//! Reader for Voikko's unweighted VFST transducer format (`mor.vfst`), ported from libvoikko's
//! `fst/UnweightedTransducer.cpp` (MPL 1.1 / GPL 2+ / LGPL 2.1+).
//!
//! Layout (little-endian): a 16-byte header (two cookies, byte 8 = weighted flag), a `u16`
//! symbol count, NUL-terminated UTF-8 symbols (epsilon, flag diacritics `@P.FEAT.VAL@`, single
//! characters, then multi-character tags like `[Ln]`), padding to 8 bytes, then 8-byte
//! transitions: `u16` input symbol, `u16` output symbol, `u32` of 24-bit target state and 8-bit
//! count of further transitions in the state (255 = the count is in the next, overflow cell).

use std::collections::HashMap;

const COOKIE1: u32 = 0x0001_3A6E;
const COOKIE2: u32 = 0x0003_51FA;
/// Traversal steps allowed per output, as in libvoikko.
const MAX_LOOP_COUNT: u32 = 100_000;
/// Stack depth and output length limit, as libvoikko's analyzer uses.
pub const BUFFER_SIZE: usize = 2000;

const FLAG_NEUTRAL: u16 = 0;
const FLAG_ANY: u16 = 1;

#[derive(Clone, Copy, Debug)]
enum Op {
    P,
    C,
    U,
    R,
    D,
}

#[derive(Clone, Copy, Debug)]
struct Diacritic {
    op: Op,
    feature: u16,
    value: u16,
}

pub struct Transducer {
    data: Box<[u8]>,
    /// Byte offset of the transition table.
    transitions: usize,
    symbols: Vec<Box<str>>,
    diacritics: Vec<Diacritic>,
    first_normal_char: u16,
    char_to_symbol: HashMap<char, u16>,
    unknown_symbol: u16,
    flag_features: u16,
}

/// Traversal state for one input (libvoikko's `Configuration`).
pub struct Configuration {
    stack_depth: usize,
    flag_depth: usize,
    input_depth: usize,
    input_length: usize,
    state_index: Vec<u32>,
    current_transition: Vec<u32>,
    input_symbols: Vec<u16>,
    output_symbols: Vec<u16>,
    flag_values: Vec<u16>,
    updated_flag_value: Vec<u16>,
    updated_flag_feature: Vec<u16>,
}

impl Configuration {
    pub fn new(t: &Transducer) -> Configuration {
        let n = BUFFER_SIZE;
        Configuration {
            stack_depth: 0,
            flag_depth: 0,
            input_depth: 0,
            input_length: 0,
            state_index: vec![0; n],
            current_transition: vec![0; n],
            input_symbols: vec![0; n],
            output_symbols: vec![0; n],
            flag_values: vec![0; t.flag_features as usize],
            updated_flag_value: vec![0; n],
            updated_flag_feature: vec![0; n],
        }
    }
}

struct Transition {
    sym_in: u16,
    sym_out: u16,
    target: u32,
    more: u32,
}

fn u16_at(d: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([d[i], d[i + 1]])
}

fn u32_at(d: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]])
}

fn diacritic(
    symbol: &str,
    features: &mut HashMap<String, u16>,
    values: &mut HashMap<String, u16>,
) -> Result<Diacritic, String> {
    let b = symbol.as_bytes();
    if b.len() <= 4 {
        return Err(format!("malformed flag diacritic {symbol}"));
    }
    let op = match b[1] {
        b'P' => Op::P,
        b'C' => Op::C,
        b'U' => Op::U,
        b'R' => Op::R,
        _ => Op::D,
    };
    let body = &symbol[3..symbol.len() - 1];
    let (feature, value) = body.split_once('.').unwrap_or((body, "@"));
    let n = features.len() as u16;
    let feature = *features.entry(feature.to_string()).or_insert(n);
    let n = values.len() as u16;
    let value = *values.entry(value.to_string()).or_insert(n);
    Ok(Diacritic { op, feature, value })
}

impl Transducer {
    pub fn new(data: Box<[u8]>) -> Result<Transducer, String> {
        if data.len() < 18 {
            return Err("transducer file is too short".into());
        }
        if u32_at(&data, 0) != COOKIE1 || u32_at(&data, 4) != COOKIE2 {
            return Err("not a little-endian VFST transducer".into());
        }
        if data[8] == 1 {
            return Err("expected an unweighted transducer".into());
        }
        let count = u16_at(&data, 16);
        let mut pos = 18;
        let mut symbols = Vec::with_capacity(count as usize);
        let mut diacritics = Vec::new();
        let mut first_normal_char = 0u16;
        let mut first_multi_char = 0u16;
        let mut char_to_symbol = HashMap::new();
        let mut features = HashMap::new();
        let mut values =
            HashMap::from([(String::new(), FLAG_NEUTRAL), ("@".to_string(), FLAG_ANY)]);
        for i in 0..count {
            let end = data[pos..]
                .iter()
                .position(|&b| b == 0)
                .ok_or("unterminated symbol")?
                + pos;
            let s = std::str::from_utf8(&data[pos..end]).map_err(|e| e.to_string())?;
            if i == 0 {
                diacritics.push(Diacritic {
                    op: Op::P,
                    feature: 0,
                    value: 0,
                });
                symbols.push(Box::from(""));
                pos += 1;
                continue;
            }
            if first_normal_char == 0 {
                if s.starts_with('@') {
                    diacritics.push(diacritic(s, &mut features, &mut values)?);
                } else {
                    first_normal_char = i;
                }
            } else if first_multi_char == 0 && s.starts_with('[') {
                first_multi_char = i;
            }
            if first_normal_char > 0
                && first_multi_char == 0
                && let Some(c) = s.chars().next()
            {
                char_to_symbol.insert(c, i);
            }
            symbols.push(Box::from(s));
            pos = end + 1;
        }
        let transitions = pos.div_ceil(8) * 8;
        if transitions > data.len() {
            return Err("transducer has no transition table".into());
        }
        Ok(Transducer {
            data,
            transitions,
            symbols,
            diacritics,
            first_normal_char,
            char_to_symbol,
            unknown_symbol: count,
            flag_features: features.len() as u16,
        })
    }

    fn transition(&self, index: u32) -> Transition {
        let at = self.transitions + index as usize * 8;
        let d = &self.data;
        if at + 8 > d.len() {
            // Corrupt index: a final-less dead end.
            return Transition {
                sym_in: 0xFFFE,
                sym_out: 0,
                target: 0,
                more: 0,
            };
        }
        let info = u32_at(d, at + 4);
        Transition {
            sym_in: u16_at(d, at),
            sym_out: u16_at(d, at + 2),
            target: info & 0x00FF_FFFF,
            more: info >> 24,
        }
    }

    fn max_tc(&self, state: u32) -> u32 {
        let head = self.transition(state);
        if head.more == 255 {
            let at = self.transitions + (state as usize + 1) * 8;
            if at + 4 <= self.data.len() {
                return u32_at(&self.data, at) + 1;
            }
        }
        head.more
    }

    /// Set up `c` for `input`; `false` when some character is not in the alphabet.
    pub fn prepare(&self, c: &mut Configuration, input: &[char]) -> bool {
        c.stack_depth = 0;
        c.flag_depth = 0;
        c.input_depth = 0;
        c.state_index[0] = 0;
        c.current_transition[0] = 0;
        c.input_length = 0;
        c.flag_values.iter_mut().for_each(|v| *v = 0);
        let mut all_known = true;
        for ch in input.iter().take(BUFFER_SIZE) {
            c.input_symbols[c.input_length] = match self.char_to_symbol.get(ch) {
                Some(&s) => s,
                None => {
                    all_known = false;
                    self.unknown_symbol
                }
            };
            c.input_length += 1;
        }
        all_known
    }

    fn flag_check(&self, c: &mut Configuration, symbol: u16) -> bool {
        if self.flag_features == 0 || symbol == 0 {
            return true;
        }
        let mut d = self.diacritics[symbol as usize];
        let current = c.flag_values[d.feature as usize];
        let mut update = false;
        match d.op {
            Op::P => update = true,
            Op::C => {
                d.value = FLAG_NEUTRAL;
                update = true;
            }
            Op::U => {
                if current != 0 {
                    if current != d.value {
                        return false;
                    }
                } else {
                    update = true;
                }
            }
            Op::R => {
                if (d.value == FLAG_ANY && current == FLAG_NEUTRAL)
                    || (d.value != FLAG_ANY && current != d.value)
                {
                    return false;
                }
            }
            Op::D => {
                if (d.value == FLAG_ANY && current != FLAG_NEUTRAL) || current == d.value {
                    return false;
                }
            }
        }
        c.updated_flag_feature[c.flag_depth] = d.feature;
        c.updated_flag_value[c.flag_depth] = current;
        if update {
            c.flag_values[d.feature as usize] = d.value;
        }
        c.flag_depth += 1;
        true
    }

    /// The next output for the prepared input, written to `out`.
    pub fn next(&self, c: &mut Configuration, out: &mut String) -> bool {
        let mut loops = 0;
        'main: while loops < MAX_LOOP_COUNT {
            loops += 1;
            let head = c.state_index[c.stack_depth];
            let mut current = c.current_transition[c.stack_depth];
            let max_tc = self.max_tc(head);
            let mut tc = current.wrapping_sub(head);
            while tc <= max_tc {
                if tc == 1 && max_tc >= 255 {
                    // Skip the overflow cell.
                    tc += 1;
                    current += 1;
                }
                let t = self.transition(current);
                if t.sym_in == 0xFFFF {
                    if c.input_depth == c.input_length {
                        out.clear();
                        for &s in &c.output_symbols[..c.stack_depth] {
                            let sym = &self.symbols[s as usize];
                            if out.len() + sym.len() + 1 >= BUFFER_SIZE * 4 {
                                return false;
                            }
                            out.push_str(sym);
                        }
                        c.current_transition[c.stack_depth] = current + 1;
                        return true;
                    }
                } else if (c.input_depth < c.input_length
                    && c.input_symbols[c.input_depth] == t.sym_in)
                    || (t.sym_in < self.first_normal_char && self.flag_check(c, t.sym_in))
                {
                    if c.stack_depth + 2 >= BUFFER_SIZE {
                        return false;
                    }
                    c.output_symbols[c.stack_depth] = if t.sym_out >= self.first_normal_char {
                        t.sym_out
                    } else {
                        0
                    };
                    c.current_transition[c.stack_depth] = current;
                    c.stack_depth += 1;
                    c.state_index[c.stack_depth] = t.target;
                    c.current_transition[c.stack_depth] = t.target;
                    if t.sym_in >= self.first_normal_char {
                        c.input_depth += 1;
                    }
                    continue 'main;
                }
                current += 1;
                tc += 1;
            }
            if c.stack_depth == 0 {
                return false;
            }
            c.stack_depth -= 1;
            let prev = self.transition(c.current_transition[c.stack_depth]).sym_in;
            if prev >= self.first_normal_char {
                c.input_depth -= 1;
            } else if self.flag_features > 0 && prev != 0 {
                c.flag_depth -= 1;
                let f = c.updated_flag_feature[c.flag_depth] as usize;
                c.flag_values[f] = c.updated_flag_value[c.flag_depth];
            }
            c.current_transition[c.stack_depth] += 1;
        }
        false
    }
}
