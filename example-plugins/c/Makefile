github-merged.so: trigger.c
	cd miss-demeanor-pluginutils && cargo build --release
	cp miss-demeanor-pluginutils/target/release/libmissdemeanor.a .
	gcc -shared -o github-merged.so -fPIC trigger.c libmissdemeanor.a

.PHONY:
clean:
	rm github-merged.so libmissdemeanor.a
