CARGO_RUBY_VERSION=ruby-2.3

default:
	cargo build --release

ruby: with-ruby

.PHONY:
with-ruby:
	CARGO_RUBY_VERSION=${CARGO_RUBY_VERSION} cargo rustc --features=ruby --release -- -C "link-args=`pkg-config --libs-only-l ${CARGO_RUBY_VERSION}`"
