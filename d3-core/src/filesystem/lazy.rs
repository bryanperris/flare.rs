/*

Instead of have types in D3 implement all these page in functions
We wrap these types into lazy loaders like:
let my_tex = LazyHogEntry<Texture>

upon its usuage, it will load data into RAM

*/