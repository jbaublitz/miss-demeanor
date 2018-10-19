#include <ruby.h>

extern int start_miss_demeanor_ruby() {
  if (ruby_setup()) {
    return -1;
  }

  ruby_finalize();
}
