# Integration Test Template

Use this template when the semantic target must be observed through userspace-visible behavior.

Place the file under:

`/root/asterinas/test/initramfs/src/apps/fs/ext2/`

Use the local harness from:

`/root/asterinas/test/initramfs/src/apps/common/test.h`

## Skeleton

```c
// SPDX-License-Identifier: MPL-2.0

#include <errno.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

#include "../../common/test.h"

#define BASE_DIR "/ext2/<topic>"

static void cleanup_path(const char *path)
{
    if (unlink(path) == -1 && errno != ENOENT) {
        // Keep cleanup strict when failure would poison later tests.
    }
}

FN_SETUP(cleanup_before_test)
{
    // Remove leftovers from previous runs.
}
END_SETUP()

FN_TEST(<semantic_case_name>)
{
    // Build only the state needed for this semantic case.

    TEST_SUCC(/* setup syscall */);
    TEST_ERRNO(/* failing syscall */, <expected_errno>);
    TEST_SUCC(/* visible postcondition */);
}
END_TEST()
```

## Preferred style

* use one file for one semantic topic
* use `FN_SETUP`, `FN_TEST`, `TEST_SUCC`, `TEST_ERRNO`, `TEST_RES`, `CHECK`, and `CHECK_WITH`
* make setup and cleanup explicit
* assert visible behavior, not hidden implementation detail
* use stable path names rooted under one test directory

## Typical integration-test targets

* pathname operations and namespace shape
* errno from syscalls
* cross-mount or cross-directory semantics
* visible persistence before and after remount
* multi-process or multi-fd visible behavior

## Notes

* the ext2 `Makefile` usually needs no change for a new `*.c` file
* if a semantic case needs helpers, keep them local to the file unless several ext2 tests already share the same helper pattern
