libgithubmerged.so: src/lib.rs
	cd miss-demeanor-pluginutils && cargo build --release
	cp miss-demeanor-pluginutils/target/release/libmissdemeanor.a .
	cargo rustc --release -- -L. -lmissdemeanor
	cp ./target/release/libgithubmerged.so .

.PHONY:
clean:
	rm libmissdemeanor.a libgithubmerged.so
