#include <stdio.h>

#include "trigger.h"

int trigger(void *request) {
	printf("%s", request_get_method(request));
	printf("%s", request_get_uri(request));
	printf("%s", request_get_body(request));
	return 0;
}
