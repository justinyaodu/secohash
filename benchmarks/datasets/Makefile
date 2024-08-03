.PHONY: all

all: \
 0_to_11111111.txt \
 00000000_to_11111111.txt \
 0_to_999.txt \
 000_to_999.txt \
 0_to_fff.txt \
 000_to_fff.txt \
 bash_builtins.txt \
 hex_64_16k.txt \
 hex_7_16k.txt \
 nodejs_builtins.txt \
 python_builtins.txt

0_to_11111111.txt:
	python -c 'for i in range(256): print(f"{i:b}")' > $@

00000000_to_11111111.txt:
	python -c 'for i in range(256): print(f"{i:08b}")' > $@

0_to_999.txt:
	seq 0 999 > $@

000_to_999.txt:
	seq --format '%03.0f' 0 999 > $@

0_to_fff.txt:
	python -c 'for i in range(4096): print(f"{i:x}")' > $@

000_to_fff.txt:
	python -c 'for i in range(4096): print(f"{i:03x}")' > $@

bash_builtins.txt:
	bash -c "compgen -bk | sort" > $@

hex_64_16k.txt:
	python -c 'import hashlib; [print(hashlib.sha256(i.to_bytes(4)).hexdigest()) for i in range(16000)]' > $@

hex_7_16k.txt:
	python -c 'import hashlib; [print(hashlib.sha256(i.to_bytes(4)).hexdigest()[:7]) for i in range(16000)]' > $@

nodejs_builtins.txt:
	node scripts/nodejs_builtins.js | sort > $@

python_builtins.txt:
	python scripts/python_builtins.py | sort > $@
