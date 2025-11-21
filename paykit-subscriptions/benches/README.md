# Adding Criterion to dev-dependencies
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "signature_verification"
harness = false

[[bench]]
name = "amount_arithmetic"
harness = false

[[bench]]
name = "serialization"
harness = false

