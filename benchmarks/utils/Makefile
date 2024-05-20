.PHONY: all

all: clean bin/shuffler

clean:
	rm -rf bin

bin/shuffler:
	cd shuffler; cargo build --release
	mkdir -p bin
	mv shuffler/target/release/shuffler bin/shuffler
