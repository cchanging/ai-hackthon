// SPDX-License-Identifier: MPL-2.0

#include "../../common/test.h"

#include <sys/wait.h>
#include <unistd.h>

FN_TEST(waitid_wstopped_must_not_consume_exited_child)
{
	int pid;
	int status;
	siginfo_t info;

	pid = CHECK(fork());
	if (pid == 0) {
		_exit(37);
	}

	usleep(200 * 1000);

	TEST_ERRNO(waitid(P_PID, pid, &info, WSTOPPED | WNOHANG), ECHILD);
	TEST_RES(waitpid(pid, &status, 0),
		 _ret == pid && WIFEXITED(status) && WEXITSTATUS(status) == 37);
}
END_TEST()
