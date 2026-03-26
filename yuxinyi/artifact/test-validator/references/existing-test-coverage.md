# Existing Test Coverage

Use this guide to map `.test` cases to current unit and integration tests.

## What Counts As Coverage

A runtime test counts as coverage only when it checks the same scenario and oracle closely enough to defend against drift.

Do not count:

* nearby tests that touch the same API but a different scenario
* tests that only exercise setup code
* tests with weaker oracles than the `.test` case requires

## Coverage Labels

Use:

* `covered by unit test`
* `covered by integration test`
* `covered partially`
* `not covered`
* `unclear`

## Evidence To Cite

For each covered or partially covered case, cite:

* exact test name
* file path
* what part of the `.test` case it covers
* what remains uncovered, if partial

## Mapping Rules

### Full coverage

Use when the existing test matches:

* same mode split
* same prestate shape
* same visible result
* same important oracle

### Partial coverage

Use when the existing test covers only part of the case, for example:

* same API but weaker oracle
* success case but not cleanup case
* buffered path but not direct path
* visible result checked, but metadata or rollback not checked

### No coverage

Use when no current test reasonably defends the case against regressions.

## Runtime Test Types

Keep at least these buckets separate:

* unit test
* integration test
* fault injection
* concurrency test

If the repository does not clearly separate them, say so and classify conservatively.
