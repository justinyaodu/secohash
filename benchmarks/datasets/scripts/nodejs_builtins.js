const names = [];
for (const name of Object.getOwnPropertyNames(globalThis)) {
  names.push(name);
  const val = globalThis[name];
  if (typeof val === "function" && "prototype" in val) {
    for (const field of Object.getOwnPropertyNames(val)) {
      names.push(`${name}.${field}`);
    }
    for (const field of Object.getOwnPropertyNames(val.prototype)) {
      names.push(`${name}.prototype.${field}`);
    }
  }
}
for (const name of names) {
  console.log(name);
}
