# ADR 012: Store radio telescope visibilities in Zarr, not FITS

Status: accepted
Date: 2026-08-04

## Context

The ⟪name|Metsähovi⟫ correlator produces ⟪unit|4TB⟫ of visibilities per night as ⟪acronym|FITS⟫ files.
Analysts open them from ⟪product|JupyterHub⟫ with ⟪crate|astropy⟫; a typical notebook reads
⟪unit|200GB⟫ to plot one baseline. ⟦its_its|Its|It's⟧ slow because ⟪acronym|FITS⟫ has no chunking and the
files sit on ⟪acronym|NFS⟫. ⟪name|Noora Heikkinen⟫ measured ⟪unit|40 minutes⟫ for a plot that
⟦agreement|need|needs⟧ ⟪unit|0.5%⟫ of the bytes.

## Decision

Write ⟪product|Zarr⟫ ⟪version|v3⟫ arrays to object storage, chunked by baseline and hour, with
⟪acronym|FITS⟫ exported on demand for archival submission. Metadata stays in the existing
⟪product|PostgreSQL⟫ catalog; ⟦its_its|it's|its⟧ schema gains a ⟪code|`zarr_url`⟫ column.

## Consequences

- The plot above ⟦agreement|take|takes⟧ ⟪unit|9 seconds⟫ in the prototype.
- Archival submission still ⟦agreement|require|requires⟧ ⟪acronym|FITS⟫; the exporter is ⟪correct|an hourly⟫
  batch job, ⟦then_than|than|then⟧ a nightly checksum pass.
- ⟪product|CASA⟫ users lose direct file access. ⟦their_there|Their|They're⟧ ⟪unit|3⟫ of them, and the
  ⟪product|Zarr⟫ to ⟪term|measurement set⟫ converter covers ⟦their_there|there|their⟧ workflow.
- Storage cost rises about ⟪unit|15%⟫ from chunk overhead. ⟦fragment|Accepted, given the analyst time saved.|This is accepted, given the analyst time saved.⟧

## Alternatives

**Keep ⟪acronym|FITS⟫, add ⟪product|Dask⟫.** Helps with parallel reads but not with the missing
chunking; every worker still reads whole ⟪acronym|HDU⟫s. Rejected after a week of trying.

**⟪acronym|HDF5⟫.** Chunked and mature, but concurrent writes from ⟪unit|8⟫ correlator nodes need
⟪acronym|MPI-IO⟫, ⟦homophone|witch|which⟧ nobody on the team wants to operate. ⟦punctuation|Reads would be fine, writes are the problem.|Reads would be fine; writes are the problem.⟧

**⟪product|Parquet⟫.** Tabular, not array-shaped. ⟦a_an|A analyst|An analyst⟧ ⟦homophone|who's|whose⟧ mental
model is a 3-D cube should not have to unpivot it. ⟦your_youre|You're|Your⟧ mileage may differ on
⟪term|calibration tables⟫, which are tabular and may move to ⟪product|Parquet⟫ later.

## Risks

⟪product|Zarr⟫ ⟪version|v3⟫ is young. We pin the ⟪crate|zarr-python⟫ version and keep the exporter
tested against ⟪version|v2⟫ so a downgrade is possible. If the spec changes in ways that
⟦homophone|effect|affect⟧ on-disk layout, ⟦its_its|its|it's⟧ a rewrite of ⟪unit|4TB⟫ per night
⟦spelling_1edit|accross|across⟧ the retention window; ⟦your_youre|you're|your⟧ estimate for that is
⟪unit|3 weeks⟫ of cluster time, so we would rather not.
