use crate::domain::RyaStatement;
use crate::resolver::triple::{TableLayout, TripleContext};
use crate::tools::mapreduce::{
    FjallKeyValue, MrConfiguration, RyaStatementInputFormat, RyaStatementRecordReader,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PigTuple {
    fields: Vec<Option<String>>,
}

impl PigTuple {
    pub fn new(fields: Vec<Option<String>>) -> Self {
        Self { fields }
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&str> {
        self.fields.get(index).and_then(Option::as_deref)
    }

    pub fn fields(&self) -> &[Option<String>] {
        &self.fields
    }
}

pub fn statement_pattern_tuple(statement: &RyaStatement) -> PigTuple {
    PigTuple::new(vec![
        Some(statement.subject.data().to_string()),
        Some(statement.predicate.data().to_string()),
        Some(statement.object.data().to_string()),
        statement
            .context
            .as_ref()
            .map(|context| context.data().to_string()),
        statement.subject.as_type().data_type().map(str::to_string),
        statement
            .predicate
            .as_type()
            .data_type()
            .map(str::to_string),
        statement.object.data_type().map(str::to_string),
    ])
}

#[derive(Clone, Debug)]
pub struct StatementPatternStorage {
    reader: RyaStatementRecordReader,
}

impl StatementPatternStorage {
    pub fn from_entries(
        context: TripleContext,
        layout: TableLayout,
        entries: Vec<FjallKeyValue>,
    ) -> Result<Self, String> {
        let mut conf = MrConfiguration::new();
        conf.set_table_layout(layout);
        let input_format = RyaStatementInputFormat::new(context);
        Ok(Self {
            reader: input_format.create_record_reader(&conf, entries)?,
        })
    }

    pub fn get_next(&mut self) -> Result<Option<PigTuple>, String> {
        if !self.reader.next_key_value()? {
            return Ok(None);
        }
        let statement = self
            .reader
            .current_value()
            .and_then(|value| value.rya_statement())
            .ok_or_else(|| {
                "StatementPatternStorage reader did not expose a statement".to_string()
            })?;
        Ok(Some(statement_pattern_tuple(statement)))
    }
}

#[cfg(test)]
#[path = "../tests/fjall_pig_tests.rs"]
mod tests;
