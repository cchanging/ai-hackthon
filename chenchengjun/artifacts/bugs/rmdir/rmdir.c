// SPDX-License-Identifier: MPL-2.0

#define _GNU_SOURCE
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

#include "../../common/test.h"

#define NON_EMPTY_DIR "/test_non_empty_dir"
#define NON_EMPTY_CHILD "/test_non_empty_dir/test.txt"
#define PERM_TEST_ROOT "/tmp/rmdir_permission_test"
#define PERM_TEST_PARENT PERM_TEST_ROOT "/parent"
#define PERM_TEST_CHILD PERM_TEST_PARENT "/child"
#define STICKY_TEST_ROOT "/tmp/rmdir_sticky_test"
#define STICKY_TEST_CHILD STICKY_TEST_ROOT "/child"
#define TEST_UID 65534

static void remove_file_if_exists(const char *path)
{
	if (unlink(path) == -1 && errno != ENOENT) {
		fprintf(stderr, "cleanup failed: unlink(%s): %s\n", path,
			strerror(errno));
		exit(EXIT_FAILURE);
	}
}

static void remove_dir_if_exists(const char *path)
{
	if (rmdir(path) == -1 && errno != ENOENT) {
		fprintf(stderr, "cleanup failed: rmdir(%s): %s\n", path,
			strerror(errno));
		exit(EXIT_FAILURE);
	}
}

static void drop_to_unprivileged_user(void)
{
	CHECK(setresgid(TEST_UID, TEST_UID, TEST_UID));
	CHECK(setresuid(TEST_UID, TEST_UID, TEST_UID));
}

FN_TEST(rmdir_failed_non_empty_dir)
{
	remove_file_if_exists(NON_EMPTY_CHILD);
	remove_dir_if_exists(NON_EMPTY_DIR);

	TEST_SUCC(mkdir(NON_EMPTY_DIR, 0777));
	TEST_SUCC(open(NON_EMPTY_CHILD, O_CREAT | O_WRONLY, 0666));

	TEST_ERRNO(rmdir(NON_EMPTY_DIR), ENOTEMPTY);

	TEST_SUCC(unlink(NON_EMPTY_CHILD));
	TEST_SUCC(rmdir(NON_EMPTY_DIR));
}
END_TEST()

FN_TEST(rmdir_requires_write_permission_on_parent)
{
	remove_dir_if_exists(PERM_TEST_CHILD);
	remove_dir_if_exists(PERM_TEST_PARENT);
	remove_dir_if_exists(PERM_TEST_ROOT);

	TEST_SUCC(mkdir(PERM_TEST_ROOT, 0755));
	TEST_SUCC(mkdir(PERM_TEST_PARENT, 0755));
	TEST_SUCC(mkdir(PERM_TEST_CHILD, 0755));
	TEST_SUCC(chmod(PERM_TEST_PARENT, 0555));

	pid_t child = TEST_SUCC(fork());
	if (child == 0) {
		drop_to_unprivileged_user();

		errno = 0;
		if (rmdir(PERM_TEST_CHILD) == -1 && errno == EACCES) {
			_exit(EXIT_SUCCESS);
		}

		_exit(EXIT_FAILURE);
	}

	int status = 0;
	TEST_RES(waitpid(child, &status, 0),
		 _ret == child && WIFEXITED(status) &&
			 WEXITSTATUS(status) == EXIT_SUCCESS);
	TEST_SUCC(access(PERM_TEST_CHILD, F_OK));

	TEST_SUCC(chmod(PERM_TEST_PARENT, 0755));
	TEST_SUCC(rmdir(PERM_TEST_CHILD));
	TEST_SUCC(rmdir(PERM_TEST_PARENT));
	TEST_SUCC(rmdir(PERM_TEST_ROOT));
}
END_TEST()

FN_TEST(rmdir_honors_sticky_bit_ownership_rules)
{
	remove_dir_if_exists(STICKY_TEST_CHILD);
	remove_dir_if_exists(STICKY_TEST_ROOT);

	TEST_SUCC(mkdir(STICKY_TEST_ROOT, 0777));
	TEST_SUCC(mkdir(STICKY_TEST_CHILD, 0755));
	TEST_SUCC(chmod(STICKY_TEST_ROOT, 01777));

	pid_t child = TEST_SUCC(fork());
	if (child == 0) {
		drop_to_unprivileged_user();

		errno = 0;
		if (rmdir(STICKY_TEST_CHILD) == -1 &&
		    (errno == EPERM || errno == EACCES)) {
			_exit(EXIT_SUCCESS);
		}

		_exit(EXIT_FAILURE);
	}

	int status = 0;
	TEST_RES(waitpid(child, &status, 0),
		 _ret == child && WIFEXITED(status) &&
			 WEXITSTATUS(status) == EXIT_SUCCESS);
	TEST_SUCC(access(STICKY_TEST_CHILD, F_OK));

	TEST_SUCC(chmod(STICKY_TEST_ROOT, 0755));
	TEST_SUCC(rmdir(STICKY_TEST_CHILD));
	TEST_SUCC(rmdir(STICKY_TEST_ROOT));
}
END_TEST()

FN_TEST(unlink_failed_on_directory)
{
	remove_dir_if_exists(NON_EMPTY_DIR);

	TEST_SUCC(mkdir(NON_EMPTY_DIR, 0777));
	TEST_ERRNO(unlink(NON_EMPTY_DIR), EISDIR);

	TEST_SUCC(rmdir(NON_EMPTY_DIR));
}
END_TEST()
