#ifndef AMBER_NAPI_HELLO_H
#define AMBER_NAPI_HELLO_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct napi_env__ *napi_env;
typedef struct napi_value__ *napi_value;
typedef struct napi_callback_info__ *napi_callback_info;
typedef napi_value (*napi_callback)(napi_env env, napi_callback_info info);
typedef int napi_status;

#define napi_ok 0

napi_status napi_create_string_utf8(napi_env env, const char *str, size_t length,
                                    napi_value *result);
napi_status napi_create_function(napi_env env, const char *utf8name, size_t length,
                                 napi_callback cb, void *data, napi_value *result);
napi_status napi_set_named_property(napi_env env, napi_value object, const char *utf8name,
                                    napi_value value);

#ifdef __cplusplus
}
#endif

#endif
