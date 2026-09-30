# ADR 012: Store radio telescope visibilities in Zarr, not FITS

Status: accepted
Date: 2026-08-04

## Context

The Metsähovi correlator produces 4TB of visibilities per night as FITS files.
Analysts open them from JupyterHub with astropy; a typical notebook reads
200GB to plot one baseline. Its slow because FITS has no chunking and the
files sit on NFS. Noora Heikkinen measured 40 minutes for a plot that
need 0.5% of the bytes.

## Decision

Write Zarr v3 arrays to object storage, chunked by baseline and hour, with
FITS exported on demand for archival submission. Metadata stays in the existing
PostgreSQL catalog; it's schema gains a `zarr_url` column.

## Consequences

- The plot above take 9 seconds in the prototype.
- Archival submission still require FITS; the exporter is an hourly
  batch job, than a nightly checksum pass.
- CASA users lose direct file access. Their 3 of them, and the
  Zarr to measurement set converter covers there workflow.
- Storage cost rises about 15% from chunk overhead. Accepted, given the analyst time saved.

## Alternatives

**Keep FITS, add Dask.** Helps with parallel reads but not with the missing
chunking; every worker still reads whole HDUs. Rejected after a week of trying.

**HDF5.** Chunked and mature, but concurrent writes from 8 correlator nodes need
MPI-IO, witch nobody on the team wants to operate. Reads would be fine, writes are the problem.

**Parquet.** Tabular, not array-shaped. A analyst who's mental
model is a 3-D cube should not have to unpivot it. You're mileage may differ on
calibration tables, which are tabular and may move to Parquet later.

## Risks

Zarr v3 is young. We pin the zarr-python version and keep the exporter
tested against v2 so a downgrade is possible. If the spec changes in ways that
effect on-disk layout, its a rewrite of 4TB per night
accross the retention window; you're estimate for that is
3 weeks of cluster time, so we would rather not.
