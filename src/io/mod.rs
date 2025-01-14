pub(crate) mod filebuf;
pub(crate) mod read;
pub(crate) mod write;

mod parse;
mod tmp2zip;
mod xmlwriter;
mod zip_out;

const DUMP_XML: bool = false;
const DUMP_UNUSED: bool = false;
