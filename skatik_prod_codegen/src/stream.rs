use crate::support::FullyQualifiedName;
use std::ops::Deref;
use truc::record::definition::RecordVariantId;

/// Defines the type of records going through a given stream.
#[derive(PartialEq, Eq, Clone, Hash, Default, Display, Debug, From)]
pub struct StreamRecordType(FullyQualifiedName);

impl Deref for StreamRecordType {
    type Target = Box<[Box<str>]>;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

/// Defines the source of a node stream, i.e. the way to connect to the source of records.
#[derive(PartialEq, Eq, Clone, Hash, Default, Display, Debug, From)]
pub struct NodeStreamSource(FullyQualifiedName);

impl Deref for NodeStreamSource {
    type Target = Box<[Box<str>]>;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

/// Node stream information
#[derive(Clone, Debug, new)]
pub struct NodeStream {
    record_type: StreamRecordType,
    variant_id: RecordVariantId,
    source: NodeStreamSource,
}

impl NodeStream {
    /// Gets the type of the records going through the entire stream.
    pub fn record_type(&self) -> &StreamRecordType {
        &self.record_type
    }

    /// Gets the record variant for a specific node.
    pub fn variant_id(&self) -> RecordVariantId {
        self.variant_id
    }

    /// Gets the source to connect to in order to read records from it.
    pub fn source(&self) -> &NodeStreamSource {
        &self.source
    }

    /// Creates a new stream with a different source.
    pub fn with_source(&self, source: NodeStreamSource) -> Self {
        Self {
            source,
            ..self.clone()
        }
    }

    /// Hack, hopefully temporary.
    pub fn definition_fragments(
        &self,
        module_prefix: &FullyQualifiedName,
    ) -> RecordDefinitionFragments {
        let prefix = format!("{}::{}::", module_prefix, self.record_type);
        let record = format!("{}Record{}", prefix, self.variant_id);
        let impl_sync_stream = format!(
            "impl SyncStream<Item = {record}::<{{ {prefix}MAX_SIZE }}>, Error = SkatikError>",
            record = record,
            prefix = prefix
        );
        let unpacked_record = format!("{}UnpackedRecord{}", prefix, self.variant_id);
        let unpacked_record_in = format!("{}UnpackedRecordIn{}", prefix, self.variant_id);
        let record_and_unpacked_out = format!("{}Record{}AndUnpackedOut", prefix, self.variant_id);
        RecordDefinitionFragments {
            prefix,
            record,
            impl_sync_stream,
            unpacked_record,
            unpacked_record_in,
            record_and_unpacked_out,
        }
    }
}

pub struct RecordDefinitionFragments {
    pub prefix: String,
    pub record: String,
    pub impl_sync_stream: String,
    pub unpacked_record: String,
    pub unpacked_record_in: String,
    pub record_and_unpacked_out: String,
}
