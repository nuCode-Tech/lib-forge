use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChecksumAlgorithm {
    Sha256,
}

impl ChecksumAlgorithm {
    pub fn as_str(self) -> &'static str {
        match self {
            ChecksumAlgorithm::Sha256 => "sha256",
        }
    }
}

impl std::fmt::Display for ChecksumAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ChecksumAlgorithm {
    type Err = ChecksumFormatError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "sha256" => Ok(ChecksumAlgorithm::Sha256),
            _ => Err(ChecksumFormatError::UnknownAlgorithm(value.to_string())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChecksumEntry {
    pub algorithm: ChecksumAlgorithm,
    pub digest: String,
    pub path: String,
}

impl ChecksumEntry {
    pub fn new(
        algorithm: ChecksumAlgorithm,
        digest: String,
        path: String,
    ) -> Result<Self, ChecksumFormatError> {
        validate_digest(algorithm, &digest)?;
        if path.trim().is_empty() {
            return Err(ChecksumFormatError::MissingPath);
        }
        Ok(Self {
            algorithm,
            digest,
            path,
        })
    }
}

pub fn render_checksum_file(entries: &[ChecksumEntry]) -> String {
    let mut sorted = entries.to_vec();
    sorted.sort_by(|left, right| match left.path.cmp(&right.path) {
        Ordering::Equal => left.digest.cmp(&right.digest),
        other => other,
    });
    sorted
        .into_iter()
        .map(|entry| format!("{} {} {}", entry.algorithm, entry.digest, entry.path))
        .collect::<Vec<String>>()
        .join("\n")
}

pub fn parse_checksum_file(contents: &str) -> Result<Vec<ChecksumEntry>, ChecksumFormatError> {
    let mut entries = Vec::new();
    for (idx, line) in contents.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.splitn(3, ' ');
        let algorithm = parts
            .next()
            .ok_or_else(|| ChecksumFormatError::InvalidLine(idx + 1))?;
        let digest = parts
            .next()
            .ok_or_else(|| ChecksumFormatError::InvalidLine(idx + 1))?;
        let path = parts
            .next()
            .ok_or_else(|| ChecksumFormatError::InvalidLine(idx + 1))?;
        let entry = ChecksumEntry::new(
            algorithm.parse()?,
            digest.to_string(),
            path.to_string(),
        )?;
        entries.push(entry);
    }
    Ok(entries)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChecksumFormatError {
    InvalidLine(usize),
    UnknownAlgorithm(String),
    InvalidDigest(String),
    MissingPath,
}

impl std::fmt::Display for ChecksumFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChecksumFormatError::InvalidLine(line) => {
                write!(f, "checksum line {} is malformed", line)
            }
            ChecksumFormatError::UnknownAlgorithm(value) => {
                write!(f, "unknown checksum algorithm '{}'", value)
            }
            ChecksumFormatError::InvalidDigest(value) => {
                write!(f, "invalid checksum digest '{}'", value)
            }
            ChecksumFormatError::MissingPath => write!(f, "checksum path is missing"),
        }
    }
}

impl std::error::Error for ChecksumFormatError {}

fn validate_digest(
    algorithm: ChecksumAlgorithm,
    digest: &str,
) -> Result<(), ChecksumFormatError> {
    match algorithm {
        ChecksumAlgorithm::Sha256 => {
            if digest.len() != 64 || !digest.chars().all(|ch| ch.is_ascii_hexdigit()) {
                return Err(ChecksumFormatError::InvalidDigest(digest.to_string()));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_is_deterministic() {
        let entries = vec![
            ChecksumEntry::new(
                ChecksumAlgorithm::Sha256,
                "b".repeat(64),
                "lib/libdemo.so".to_string(),
            )
            .expect("entry"),
            ChecksumEntry::new(
                ChecksumAlgorithm::Sha256,
                "a".repeat(64),
                "metadata/manifest.json".to_string(),
            )
            .expect("entry"),
        ];
        let rendered = render_checksum_file(&entries);
        let expected = format!(
            "sha256 {} metadata/manifest.json\nsha256 {} lib/libdemo.so",
            "a".repeat(64),
            "b".repeat(64)
        );
        assert_eq!(rendered, expected);
    }

    #[test]
    fn parse_round_trips() {
        let contents = format!(
            "sha256 {} metadata/manifest.json\nsha256 {} lib/libdemo.so",
            "a".repeat(64),
            "b".repeat(64)
        );
        let entries = parse_checksum_file(&contents).expect("parse");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "metadata/manifest.json");
        assert_eq!(entries[1].path, "lib/libdemo.so");
    }
}
