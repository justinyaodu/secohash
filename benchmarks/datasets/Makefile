.PHONY: all

all: 0_to_999.txt 000_to_999.txt 0_to_fff.txt 000_to_fff.txt bash_builtins.txt nodejs_builtins.txt python_builtins.txt

0_to_999.txt:
	seq 0 999 > $@

000_to_999.txt:
	seq --format '%03.0f' 0 999 > $@

0_to_fff.txt:
	bash -c 'for (( i = 0; i < 4096; i++ )); do printf "%x\n" "$$i"; done' > $@

000_to_fff.txt:
	bash -c 'for (( i = 0; i < 4096; i++ )); do printf "%03x\n" "$$i"; done' > $@

bash_builtins.txt:
	bash -c "compgen -bk | sort" > $@

nodejs_builtins.txt:
	node scripts/nodejs_builtins.js | sort > $@

python_builtins.txt:
	python scripts/python_builtins.py | sort > $@
