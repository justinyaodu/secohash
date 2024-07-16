.PHONY: all

all: bash_builtins.txt 0_to_999.txt 000_to_999.txt 0_to_fff.txt 000_to_fff.txt

bash_builtins.txt:
	bash -c "compgen -bk | sort" > $@

0_to_999.txt:
	seq 0 999 > $@

000_to_999.txt:
	seq --format '%03.0f' 0 999 > $@

0_to_fff.txt:
	bash -c 'for (( i = 0; i < 4096; i++ )); do printf "%x\n" "$$i"; done' > $@

000_to_fff.txt:
	bash -c 'for (( i = 0; i < 4096; i++ )); do printf "%03x\n" "$$i"; done' > $@
