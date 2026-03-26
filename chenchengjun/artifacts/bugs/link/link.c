// SPDX-License-Identifier: MPL-2.0

#define _GNU_SOURCE

#include <errno.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

#include "../../common/test.h"

#define TEST_ROOT "/tmp/link_test"
#define SOURCE_PATH TEST_ROOT "/source"
#define TARGET_DIR TEST_ROOT "/target"
#define LINK_PATH TARGET_DIR "/linked"
#define TEST_UID 65534

static void remove_file_if_exists(const char *path)
{
	if (unlink(path) == -1 && errno != ENOENT) {
		perror("unlink");
		exit(EXIT_FAILURE);
	}
}

static void remove_dir_if_exists(const char *path)
{
	if (rmdir(path) == -1 && errno != ENOENT) {
		perror("rmdir");
		exit(EXIT_FAILURE);
	}
}

static void drop_to_unprivileged_user(void)
{
	CHECK(setresgid(TEST_UID, TEST_UID, TEST_UID));
	CHECK(setresuid(TEST_UID, TEST_UID, TEST_UID));
}

FN_TEST(link_requires_write_permission_on_new_parent)
{
	remove_file_if_exists(LINK_PATH);
	remove_dir_if_exists(TARGET_DIR);
	remove_file_if_exists(SOURCE_PATH);
	remove_dir_if_exists(TEST_ROOT);

	TEST_SUCC(mkdir(TEST_ROOT, 0755));
	TEST_SUCC(mkdir(TARGET_DIR, 0755));

	int fd = TEST_SUCC(open(SOURCE_PATH, O_CREAT | O_RDWR | O_TRUNC, 0644));
	TEST_SUCC(close(fd));
	TEST_SUCC(chown(SOURCE_PATH, TEST_UID, TEST_UID));
	TEST_SUCC(chmod(TARGET_DIR, 0555));

	pid_t child = TEST_SUCC(fork());
	if (child == 0) {
		drop_to_unprivileged_user();

		errno = 0;
		if (link(SOURCE_PATH, LINK_PATH) == -1 && errno == EACCES) {
			_exit(EXIT_SUCCESS);
		}

		_exit(EXIT_FAILURE);
	}

	int status = 0;
	TEST_RES(waitpid(child, &status, 0),
		 _ret == child && WIFEXITED(status) &&
			 WEXITSTATUS(status) == EXIT_SUCCESS);
	TEST_ERRNO(access(LINK_PATH, F_OK), ENOENT);

	TEST_SUCC(chmod(TARGET_DIR, 0755));
	remove_file_if_exists(LINK_PATH);
	TEST_SUCC(unlink(SOURCE_PATH));
	TEST_SUCC(rmdir(TARGET_DIR));
	TEST_SUCC(rmdir(TEST_ROOT));
}
END_TEST()
