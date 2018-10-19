#include <ruby.h>
#include "ruby/miss-demeanor-pluginutils.h"

extern int start_miss_demeanor_ruby() {
        if (ruby_setup()) {
                return -1;
        }

        return 0;
}

extern void *run_ruby_trigger(void *request) {
	char *method = hyper_request_method(request);
	return NULL;
}

extern void cleanup_miss_demeanor_ruby() {
	ruby_finalize();
}
