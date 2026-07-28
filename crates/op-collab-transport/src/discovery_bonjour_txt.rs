use super::DiscoveryError;

pub(super) fn decode_txt_record(txt: &[u8]) -> Result<Vec<(&str, &str)>, DiscoveryError> {
    let mut entries = Vec::with_capacity(3);
    let mut remaining = txt;
    while let Some((&length, tail)) = remaining.split_first() {
        let length = usize::from(length);
        if length == 0 || tail.len() < length || entries.len() == 3 {
            return Err(DiscoveryError::InvalidMetadata);
        }
        let entry =
            std::str::from_utf8(&tail[..length]).map_err(|_| DiscoveryError::InvalidMetadata)?;
        let (key, value) = entry
            .split_once('=')
            .ok_or(DiscoveryError::InvalidMetadata)?;
        if key.is_empty() || value.is_empty() || value.contains('=') {
            return Err(DiscoveryError::InvalidMetadata);
        }
        entries.push((key, value));
        remaining = &tail[length..];
    }
    if entries.len() != 3 {
        return Err(DiscoveryError::InvalidMetadata);
    }
    Ok(entries)
}

pub(super) fn property_from_pairs<'a>(
    properties: &'a [(&str, &str)],
    key: &str,
) -> Result<&'a str, DiscoveryError> {
    let mut values = properties
        .iter()
        .filter_map(|(candidate, value)| (*candidate == key).then_some(*value));
    let value = values.next().ok_or(DiscoveryError::InvalidMetadata)?;
    if values.next().is_some()
        || properties
            .iter()
            .any(|(candidate, _)| !matches!(*candidate, "id" | "v" | "p"))
    {
        return Err(DiscoveryError::InvalidMetadata);
    }
    Ok(value)
}
