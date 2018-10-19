require 'mkmf'

create_header
create_makefile ENV['CARGO_PKG_NAME']
