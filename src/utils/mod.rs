use std::iter;

use radix;
use rand::{distributions::Alphanumeric, prelude::thread_rng, Rng};

pub fn create_slug_from_id(id: i32) -> String {
    let id = format!("{}", id);
    let r = radix::RadixNum::from_str(&id, 10).unwrap();
    let r = r.with_radix(36).unwrap();
    let slug = r.as_str();
    if 6i32 - slug.len() as i32 > 0 {
        let len = 6 - slug.len();
        let mut rng = thread_rng();
        return format!(
            "{}{}",
            slug,
            iter::repeat(())
                .map(|()| rng.sample(Alphanumeric))
                .take(len)
                .collect::<String>()
        );
    }

    slug.to_string()
}

#[cfg(test)]
mod tests {
    use super::create_slug_from_id;

    #[test]
    fn returns_length_of_atleast_six() {
        let slug = create_slug_from_id(10);
        assert_eq!(slug.chars().next().unwrap(), 'A');
        assert_eq!(slug.len(), 6);
    }

    #[test]
    fn returns_full_string_if_six_length() {
        let slug = create_slug_from_id(439483745);
        assert_eq!(slug, "79NNTT");
    }
}
