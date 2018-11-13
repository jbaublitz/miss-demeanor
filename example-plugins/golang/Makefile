github-merged.so: trigger.go
	cd miss-demeanor-pluginutils && cargo build --release
	cp miss-demeanor-pluginutils/target/release/libmissdemeanor.a .
	go build -buildmode=c-shared -o github-merged.so trigger.go

.PHONY:
clean:
	rm github-merged.so libmissdemeanor.a github-merged.h
