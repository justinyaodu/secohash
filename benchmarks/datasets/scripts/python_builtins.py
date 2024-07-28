names = []

for name, val in vars(__builtins__).items():
    names.append(name)
    if type(val) == type:
        for field in vars(val).keys():
            names.append(f"{name}.{field}")

mod_names = [
    "abc",
    "collections",
    "copy",
    "csv",
    "dataclasses",
    "datetime",
    "decimal",
    "enum",
    "fractions",
    "functools",
    "hashlib",
    "html",
    "io",
    "itertools",
    "json",
    "math",
    "operator",
    "os",
    "pathlib",
    "random",
    "re",
    "statistics",
    "string",
    "sys",
    "textwrap",
    "types",
    "typing",
    "unittest",
]
for mod_name in mod_names:
    mod = __import__(mod_name)
    for name, val in vars(mod).items():
        names.append(f"{mod_name}.{name}")
        if type(val) == type:
            for field in vars(val).keys():
                names.append(f"{mod_name}.{name}.{field}")

for name in names:
    print(name)
