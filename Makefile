RUBY_VERSION?=ruby-2.5

default:
	cargo build --release

ruby:
	cargo rustc --release --features=ruby -- -C "link-args=`pkg-config --libs ${RUBY_VERSION}`"
