#include <ruby.h>
#include <stdlib.h>

#include "miss-demeanor-pluginutils.h"

VALUE evaluator_class;

struct request_pointer {
	void *request;
};

size_t request_size(const void *data) {
	(void)data;
	return sizeof(struct request_pointer);
}

static const rb_data_type_t request_t = {
	.wrap_struct_name = "request",
	.function = {
		.dmark = NULL,
		.dfree = RUBY_DEFAULT_FREE,
		.dsize = request_size,
	},
	.data = NULL,
	.flags = RUBY_TYPED_FREE_IMMEDIATELY,
};

VALUE wrap_no_args(VALUE (*func)(VALUE), VALUE self) {
	int state;
	VALUE result = rb_protect(func, self, &state);
	if (state) {
		return Qnil;
	} else {
		return result;
	}
}

VALUE request_method_c(VALUE self) {
	struct request_pointer *data;
	TypedData_Get_Struct(self, struct request_pointer, &request_t, data);

	int strlen;
	char *str = hyper_request_method(data->request, &strlen);
	if (!str) {
		return Qnil;
	}
	return rb_str_new(str, strlen);
}

VALUE request_uri_c(VALUE self) {
	struct request_pointer *data;
	TypedData_Get_Struct(self, struct request_pointer, &request_t, data);

	int strlen;
	char *str = hyper_request_uri(data->request, &strlen);
	if (!str) {
		return Qnil;
	}
	return rb_str_new(str, strlen);
}

VALUE request_get_body_c(VALUE self) {
	struct request_pointer *data;
	TypedData_Get_Struct(self, struct request_pointer, &request_t, data);

	int strlen;
	char *str = hyper_request_get_body(data->request, &strlen);
	if (!str) {
		return Qnil;
	}
	return rb_str_new(str, strlen);
}

VALUE request_method(VALUE self) {
	return wrap_no_args(request_method_c, self);
}

VALUE request_uri(VALUE self) {
	return wrap_no_args(request_uri_c, self);
}

VALUE request_body(VALUE self) {
	return wrap_no_args(request_get_body_c, self);
}

VALUE request_alloc(VALUE self) {
	struct request_pointer *data;
	return TypedData_Make_Struct(self, struct request_pointer, &request_t, data);
}

extern VALUE run_ruby_trigger(char *ruby_path, void *request) {
	VALUE instance = rb_funcall(evaluator_class, rb_intern("new"), 0);
	rb_include_module(instance, rb_intern("Plugin"));

	VALUE script = rb_str_new_cstr(ruby_path);
	int state;
	rb_load_protect(script, 0, &state);

	if (state) {
		return Qnil;
	}

	struct request_pointer *data;
	TypedData_Get_Struct(instance, struct request_pointer, &request_t, data);
	data->request = request;

	return rb_funcall(instance, rb_intern("run_trigger"), 0);
}

extern int start_miss_demeanor_ruby() {
        if (ruby_setup()) {
                return -1;
        }

	evaluator_class = rb_define_class("MissDemeanorRuby", rb_cData);

	rb_define_alloc_func(evaluator_class, request_alloc);

        return 0;
}

extern void cleanup_miss_demeanor_ruby() {
	ruby_finalize();
}

extern int is_nil(unsigned long id) {
	return NIL_P(id);
}
