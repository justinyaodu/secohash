.PHONY: all

all: bash_builtins.txt 0_to_999.txt 000_to_999.txt

bash_builtins.txt:
	bash -c "compgen -bk | sort" > $@

0_to_999.txt:
	seq 0 999 > $@

000_to_999.txt:
	seq --format '%03.0f' 0 999 > $@
