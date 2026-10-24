#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct DomainInput {
    #[arbitrary(with = "arbitrary_domain_string")]
    domain: String,
}

fn arbitrary_domain_string(u: &mut arbitrary::Unstructured) -> arbitrary::Result<String> {
    // Generate a valid UTF-8 string that could be a domain
    let labels: Vec<String> = u
        .arbitrary_iter()?
        .take(u.int_in_range(1..=5)?)
        .collect::<arbitrary::Result<_>>()?;

    let domains: Vec<String> = labels
        .iter()
        .map(|label| {
            // Ensure each label is valid: alphanumeric and hyphens, no leading/trailing hyphen
            label
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-')
                .collect::<String>()
        })
        .filter(|label| !label.is_empty() && !label.starts_with('-') && !label.ends_with('-'))
        .collect();

    if domains.is_empty() {
        Ok("example.com".to_string())
    } else {
        Ok(domains.join("."))
    }
}

fuzz_target!(|input: DomainInput| {
    // Must not panic regardless of input
    let _ = anchorkit::domain_validator::validate_anchor_domain(&input.domain);

    // Batch variant must also be panic-free and return one result per input
    let inputs: Vec<&str> = input.domain.lines().collect();
    let results = anchorkit::domain_validator::validate_anchor_domain_batch(&inputs);
    assert_eq!(results.len(), inputs.len());
});
