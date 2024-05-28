.PHONY: all

all: bash_builtins.txt three_digit_numbers.txt

bash_builtins.txt:
	bash -c "compgen -bk | sort" > $@

three_digit_numbers.txt:
	seq --format '%03.0f' 0 999 > $@
