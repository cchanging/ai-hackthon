// SPDX-License-Identifier: MPL-2.0

#define _GNU_SOURCE

#include "../../common/test.h"

#include <fcntl.h>
#include <sys/syscall.h>
#include <sys/types.h>
#include <unistd.h>

static int pipe_read_fd;
static int pipe_write_fd;
static int ro_fd;
static int rw_fd;

FN_SETUP(open_fds)
{
	int pipe_fds[2];

	CHECK(pipe(pipe_fds));
	pipe_read_fd = pipe_fds[0];
	pipe_write_fd = pipe_fds[1];

	ro_fd = CHECK(open("/etc/passwd", O_RDONLY));
	rw_fd = CHECK(open("/tmp/pwrite64_zero_len", O_CREAT | O_RDWR | O_TRUNC, 0644));
}
END_SETUP()

FN_TEST(pwrite64_zero_len_pipe_returns_espipe)
{
	TEST_ERRNO(syscall(SYS_pwrite64, pipe_write_fd, NULL, 0, (off_t)0), ESPIPE);
}
END_TEST()

FN_TEST(pwrite64_zero_len_readonly_returns_ebadf)
{
	TEST_ERRNO(syscall(SYS_pwrite64, ro_fd, NULL, 0, (off_t)0), EBADF);
}
END_TEST()

FN_TEST(pwrite64_zero_len_ignores_user_ptr)
{
	// When `count == 0`, Linux does not dereference the buffer pointer.
	void *bad_ptr = (void *)1;
	TEST_RES(syscall(SYS_pwrite64, rw_fd, bad_ptr, 0, (off_t)0), _ret == 0);
}
END_TEST()

FN_TEST(pwrite64_zero_len_negative_offset_returns_einval)
{
	TEST_ERRNO(syscall(SYS_pwrite64, rw_fd, NULL, 0, (off_t)-1), EINVAL);
}
END_TEST()

FN_SETUP(cleanup)
{
	CHECK(close(pipe_read_fd));
	CHECK(close(pipe_write_fd));
	CHECK(close(ro_fd));
	CHECK(close(rw_fd));
	CHECK(unlink("/tmp/pwrite64_zero_len"));
}
END_SETUP()

