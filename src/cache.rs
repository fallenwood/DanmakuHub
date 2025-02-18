// sqlite should be fast enough, keep cache unimplemented for now
pub fn get_or_set(key: String, setter: impl FnOnce() -> String) -> String {
  todo!("Implement get_or_set")
}
