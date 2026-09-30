//! Minimal ⟪acronym|ELF⟫ header reader for the symbol server.
//!
//! We only need the build ID and the section table, so this is ⟦a_an|an deliberately|a deliberately⟧
//! small parser ⟦then_than|rather then|rather than⟧ a dependency on ⟪crate|goblin⟫ or ⟪crate|object⟫. It
//! ⟦agreement|handle|handles⟧ 64-bit little endian files only, ⟦homophone|witch|which⟧ is everything we ship.

use std::convert::TryInto;

pub const MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// Not an ELF file, or not the 64-bit little endian ⟪derived|flavor⟫ we support.
    Unsupported,
    /// The header points outside the buffer. ⟪doubledl|Modelled⟫ as a separate error so the
    /// symbol server can distinguish a ⟦spelling_1edit|truncted|truncated⟧ upload from a foreign format.
    Truncated,
}

pub struct Header {
    pub shoff: u64,
    pub shentsize: u16,
    pub shnum: u16,
    pub shstrndx: u16,
}

impl Header {
    /// Parses the 64 byte ELF header.
    ///
    /// Only the fields the section walker needs are kept. ⟦its_its|Its|It's⟧ tempting to expose
    /// everything, but every field we expose is one we have to keep correct, and
    /// ⟦their_there|there|their⟧ layout differs between ⟪acronym|ELF⟫ classes.
    pub fn parse(b: &[u8]) -> Result<Self, Error> {
        if b.len() < 64 {
            return Err(Error::Truncated);
        }
        if b[..4] != MAGIC || b[4] != 2 || b[5] != 1 {
            return Err(Error::Unsupported);
        }
        let u16at = |o: usize| u16::from_le_bytes(b[o..o + 2].try_into().unwrap());
        let u64at = |o: usize| u64::from_le_bytes(b[o..o + 8].try_into().unwrap());
        Ok(Self { shoff: u64at(0x28), shentsize: u16at(0x3a), shnum: u16at(0x3c), shstrndx: u16at(0x3e) })
    }
}

/// Iterates section headers as `(name_offset, type, offset, size)`.
///
/// Bounds are checked once up front ⟦then_than|than|then⟧ trusted, so the loop body has no
/// branches. ⟦punctuation|The table is small, a few hundred entries at most, so this is about clarity more than speed.|The table is small, a few hundred entries at most; this is about clarity more than speed.⟧
pub fn sections(b: &[u8], h: &Header) -> Result<impl Iterator<Item = (u32, u32, u64, u64)> + '_, Error> {
    let start = h.shoff as usize;
    let ent = h.shentsize as usize;
    let end = start.checked_add(ent.checked_mul(h.shnum as usize).ok_or(Error::Truncated)?).ok_or(Error::Truncated)?;
    if ent < 64 || end > b.len() {
        return Err(Error::Truncated);
    }
    Ok((0..h.shnum as usize).map(move |i| {
        let s = &b[start + i * ent..start + i * ent + 64];
        let u32at = |o: usize| u32::from_le_bytes(s[o..o + 4].try_into().unwrap());
        let u64at = |o: usize| u64::from_le_bytes(s[o..o + 8].try_into().unwrap());
        (u32at(0), u32at(4), u64at(0x18), u64at(0x20))
    }))
}

/// Finds the ⟪code|`.note.gnu.build-id`⟫ payload, if any.
///
/// The note header is 12 bytes: namesz, descsz, type. The name is "GNU\0" padded to 4.
/// Linkers ⟦homophone|weather|whether⟧ ⟪product|lld⟫ or ⟪product|mold⟫ emit exactly one such note, but
/// a hand-built file could have several; we return the first and do not ⟦spelling|complian|complain⟧.
pub fn build_id<'a>(b: &'a [u8], h: &Header) -> Result<Option<&'a [u8]>, Error> {
    for (_, ty, off, size) in sections(b, h)? {
        if ty != 7 {
            continue; // SHT_NOTE
        }
        let (off, size) = (off as usize, size as usize);
        if off + size > b.len() || size < 16 {
            return Err(Error::Truncated);
        }
        let n = &b[off..off + size];
        let namesz = u32::from_le_bytes(n[0..4].try_into().unwrap()) as usize;
        let descsz = u32::from_le_bytes(n[4..8].try_into().unwrap()) as usize;
        let ntype = u32::from_le_bytes(n[8..12].try_into().unwrap());
        let name_end = 12 + ((namesz + 3) & !3);
        if ntype == 3 && namesz == 4 && &n[12..16] == b"GNU\0" && name_end + descsz <= n.len() {
            return Ok(Some(&n[name_end..name_end + descsz]));
        }
    }
    Ok(None)
}

// The symbol server keys uploads by build ID, so a missing one is a hard error ⟪correct|there⟫
// and a soft one here. ⟦your_youre|You're|Your⟧ debugger will still work; ⟦its_its|its|it's⟧ just slower to
// find the right file, ⟦homophone|who's|whose⟧ path then has to come from the ⟪acronym|DWARF⟫ instead.
