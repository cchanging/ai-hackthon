# Gap Detection

After checking traceability, run a second pass that asks what the `.test` still misses.

## Common `.test` Gaps

Look for:

* missing spec obligations
* missing mode splits
* missing boundary cases
* missing failure or cleanup cases
* missing metadata or persistence oracles
* missing static-review obligations
* missing fault-injection or concurrency cases
* cases that are too broad to implement reliably
* cases with weak traceability

## Gap Types

Classify gaps as:

* `missing case`
* `missing mode split`
* `weak oracle`
* `weak traceability`
* `missing validation mode`
* `case too broad`

## What To Recommend

When you find a gap, recommend a concrete addition:

* a new case title
* a stronger oracle
* a missing validation mode
* a missing traceability clause

## Filesystem Bias

For filesystem-style `.test` artifacts, always check for missing coverage in:

* zero length
* EOF transitions
* partial completion
* cache coherence
* metadata updates
* rollback after failed extension
* visibility vs durability
* cross-API consistency
* concurrency-sensitive corners
