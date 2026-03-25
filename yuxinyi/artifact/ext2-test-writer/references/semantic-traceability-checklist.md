# Semantic Traceability Checklist

Before finishing a test-writing task, check each item:

* Every added test maps to at least one explicit spec or semantic obligation.
* Each important MUST-level obligation is covered by either a unit test, an integration test, or is explicitly deferred.
* Unit tests cover local invariants rather than public behavior that should be checked through syscalls.
* Integration tests cover public behavior rather than trying to infer internal state indirectly.
* Errno expectations match the spec or Linux-visible reference used by the task.
* Persistence or remount semantics are tested only when they are part of the requirement.
* Cleanup prevents one test from poisoning the next run.
* Test names describe the semantic scenario, not just the syscall name.
* Assertions prove the intended semantic contract and avoid unnecessary implementation coupling.
* The final answer states what remains for xfstests or another end-to-end harness.
