# Xfstests Fix Session Context

Use this file as the fixed root-directory and default-command reference for xfstests repair rounds.

Normal usage only needs a `failing_case`. The skill derives the concrete artifact paths from the case name and the roots below.

Use this file to pin the Trellis task for the round when needed. If `trellis_task` is present, reuse it instead of guessing.

Resolution rules:

- `error_log_path` is the root xfstests error-log directory.
- The concrete case log is `<error_log_path>/<failing_case>.log`.
- `report_path` is the root report directory.
- Normalize `failing_case` into `case_slug` by replacing `/` with `-`.
- The concrete report artifact is the lexicographically last file in `report_path` whose basename contains `case_slug`.
- Example: `generic/001` resolves to `/root/xfstests/error_log/generic/001.log`.
- Example: `generic/038` matches `/root/asterinas/.trellis/reports/xfstests/xfstests-analysis-2026-03-19-092822-generic-038.md`.
- If multiple reports match one case, pick the lexicographically last match. This follows the timestamped naming convention and therefore selects the newest report.

- failing_case: `<generic/029>`
- trellis_task: `<optional task dir, task name, or tracker such as 03-24-xfstests-fix-master-tracker>`
- error_log_path: `</root/xfstests/error_log/>`
- report_path: `</root/asterinas/.trellis/reports/xfstests/>`
- manual_validation_command: `<optional override; otherwise derive make run_kernel AUTO_TEST=xfstests RELEASE=1 MEM=12G XFSTESTS_ARGS='<failing_case>'>`
  example: `make run_kernel AUTO_TEST=xfstests RELEASE=1 MEM=12G XFSTESTS_ARGS='generic/001'`
- follow_up_error_log_path: `</root/asterinas/qemu.log>`
