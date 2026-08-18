# Security Policy

ZstdScope parses untrusted binary input, so parser safety is a core project requirement.

## Supported versions

ZstdScope is currently pre-release and has no supported published version yet. This section will be updated when releases begin.

## Reporting a vulnerability

Please do not include exploit details or proof-of-concept payloads in a public issue when the problem could affect users.

Preferred reporting path:

1. use GitHub's private vulnerability reporting / Security Advisory mechanism for this repository when available;
2. if private reporting is not available, open a minimal public issue asking the maintainer for a private contact path **without including vulnerability details**.

Useful information in a private report includes:

- affected commit or version;
- input conditions required to trigger the problem;
- impact, such as panic, denial of service, excessive allocation, or incorrect bounds handling;
- reproduction steps or a minimized test case;
- whether the issue is believed to be exploitable beyond a parser crash.

## Security assumptions

ZstdScope assumes all inspected bytes can be attacker controlled.

Security-sensitive parser properties include:

- no out-of-bounds reads;
- no integer overflow in offset/size calculations;
- no panic caused by malformed input;
- no allocation proportional to an untrusted declared payload merely to skip it;
- bounded and understandable metadata allocation behavior;
- explicit rejection of reserved/invalid structural values.

## Fuzzing

Fuzz testing is planned as part of the parser's quality strategy. The baseline invariant is:

```text
any byte sequence -> successful inspection or typed error, never panic
```

Security-relevant regression cases should remain in the permanent test corpus after a fix.
