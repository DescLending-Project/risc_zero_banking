// Direct test module at the root level
#[cfg(test)]
mod tests;

include!(concat!(env!("OUT_DIR"), "/methods.rs"));
