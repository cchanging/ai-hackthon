// SPDX-License-Identifier: MPL-2.0

#define _GNU_SOURCE

#include <dirent.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <unistd.h>

#include "../../common/test.h"

static ino_t get_dirent_ino(const char *dir_path, const char *entry_name);
static ino_t get_stat_ino(const char *dir_path, const char *entry_name, int flags);

#define TEST_DIRENT_INO_MATCH(dir_path, entry_name, stat_flags)               \
	do {                                                                  \
		ino_t dirent_ino =                                            \
			TEST_RES(get_dirent_ino((dir_path), (entry_name)),    \
				 _ret != 0);                                 \
		if (dirent_ino != 0) {                                        \
			TEST_RES(get_stat_ino((dir_path), (entry_name),       \
					 stat_flags),                         \
				 _ret == dirent_ino);                         \
		}                                                             \
	} while (0)

FN_TEST(readdir_inode_matches_lookup)
{
	char pid_name[32];
	char tid_name[32];

	TEST_RES(snprintf(pid_name, sizeof(pid_name), "%d", getpid()),
		 _ret > 0 && _ret < (int)sizeof(pid_name));
	TEST_RES(snprintf(tid_name, sizeof(tid_name), "%ld",
			  syscall(SYS_gettid)),
		 _ret > 0 && _ret < (int)sizeof(tid_name));

	TEST_DIRENT_INO_MATCH("/proc", "meminfo", 0);
	TEST_DIRENT_INO_MATCH("/proc", "self", AT_SYMLINK_NOFOLLOW);
	TEST_DIRENT_INO_MATCH("/proc", pid_name, 0);
	TEST_DIRENT_INO_MATCH("/proc/self", "task", 0);
	TEST_DIRENT_INO_MATCH("/proc/self/task", tid_name, 0);
	TEST_DIRENT_INO_MATCH("/proc/self", "exe", AT_SYMLINK_NOFOLLOW);
	TEST_DIRENT_INO_MATCH("/proc/self", "ns", 0);
	TEST_DIRENT_INO_MATCH("/proc/self/fd", "0", AT_SYMLINK_NOFOLLOW);
}
END_TEST()

static ino_t get_dirent_ino(const char *dir_path, const char *entry_name)
{
	DIR *dir = opendir(dir_path);
	if (dir == NULL) {
		return 0;
	}

	struct dirent *entry;
	while ((entry = readdir(dir)) != NULL) {
		if (strcmp(entry->d_name, entry_name) == 0) {
			ino_t ino = entry->d_ino;
			CHECK(closedir(dir));
			return ino;
		}
	}

	int ret = closedir(dir);
	if (ret < 0) {
		return 0;
	}

	errno = ENOENT;
	return 0;
}

static ino_t get_stat_ino(const char *dir_path, const char *entry_name, int flags)
{
	int dir_fd = open(dir_path, O_RDONLY | O_DIRECTORY);
	if (dir_fd < 0) {
		return 0;
	}

	struct stat stat_buf;
	int ret = fstatat(dir_fd, entry_name, &stat_buf, flags);
	int stat_errno = errno;
	if (close(dir_fd) < 0) {
		return 0;
	}
	if (ret < 0) {
		errno = stat_errno;
		return 0;
	}

	return stat_buf.st_ino;
}
