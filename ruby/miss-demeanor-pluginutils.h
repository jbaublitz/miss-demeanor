#ifndef MISS_DEMEANOR_PLUGINUTILS_H
#define MISS_DEMEANOR_PLUGINUTILS_H

char *hyper_request_method(const void *, int *);
char *hyper_request_uri(const void *, int *);
char *hyper_request_get_body(const void *, int *);
char *hyper_request_free_body(const void *);
char *hyper_request_get_header(const void *, const char *, int, int *);

#endif
