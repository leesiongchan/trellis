//! Versioned schema. Indexed identity fields stay outside flexible domain payloads.
use super::*;
use crate::contracts::library::LibraryId;
pub(crate) const SCHEMA_VERSION: u64 = 1;
impl KnowledgeStore {
    pub async fn install_schema(&self) -> Result<(), AppError> {
        let mut sql = String::from(
            "BEGIN TRANSACTION; DEFINE TABLE IF NOT EXISTS library SCHEMAFULL; DEFINE FIELD IF NOT EXISTS key ON library TYPE string; DEFINE FIELD IF NOT EXISTS epoch ON library TYPE int; DEFINE FIELD IF NOT EXISTS schemaVersion ON library TYPE int; ",
        );
        let tables: &[(&str, &[(&str, &str)])] = &[
            ("topic", &[("key", "string")]),
            (
                "question_variant",
                &[
                    ("key", "string"),
                    ("exactKey", "string"),
                    ("topicKey", "string"),
                ],
            ),
            (
                "exploration",
                &[
                    ("key", "string"),
                    ("revision", "int"),
                    ("updatedAt", "string"),
                ],
            ),
            (
                "appearance",
                &[
                    ("key", "string"),
                    ("explorationKey", "string"),
                    ("topicKey", "string"),
                    ("parentKey", "option<string | null>"),
                    ("answerVersionKey", "option<string | null>"),
                ],
            ),
            (
                "answer_version",
                &[
                    ("key", "string"),
                    ("topicKey", "string"),
                    ("revision", "int"),
                ],
            ),
            (
                "operation",
                &[
                    ("key", "string"),
                    ("requestHash", "string"),
                    ("libraryEpoch", "int"),
                    ("target", "string"),
                ],
            ),
            (
                "research_attempt",
                &[
                    ("key", "string"),
                    ("topicKey", "string"),
                    ("operationKey", "string"),
                ],
            ),
            (
                "active_attempt",
                &[
                    ("key", "string"),
                    ("topicKey", "string"),
                    ("attemptKey", "string"),
                ],
            ),
            (
                "edge_support",
                &[
                    ("key", "string"),
                    ("connectionKey", "string"),
                    ("appearanceKey", "string"),
                ],
            ),
            (
                "embedding",
                &[
                    ("key", "string"),
                    ("modelVersion", "string"),
                    ("sourceKey", "string"),
                    ("inputHash", "string"),
                ],
            ),
            (
                "migration_manifest",
                &[("key", "string"), ("identityKey", "string")],
            ),
            (
                "migration_stage",
                &[
                    ("key", "string"),
                    ("importKey", "string"),
                    ("batchIndex", "int"),
                ],
            ),
            (
                "import_receipt",
                &[("key", "string"), ("identityKey", "string")],
            ),
        ];
        for (table, fields) in tables {
            sql.push_str(&format!("DEFINE TABLE IF NOT EXISTS {table} SCHEMAFULL; DEFINE FIELD IF NOT EXISTS data ON {table} TYPE object FLEXIBLE; "));
            for (field, kind) in *fields {
                sql.push_str(&format!(
                    "DEFINE FIELD IF NOT EXISTS {field} ON {table} TYPE {kind}; "
                ));
            }
            sql.push_str(&format!(
                "DEFINE INDEX IF NOT EXISTS unique_key ON {table} FIELDS key UNIQUE; "
            ));
        }
        sql.push_str("DEFINE TABLE IF NOT EXISTS connection TYPE RELATION IN topic OUT topic SCHEMAFULL; DEFINE FIELD IF NOT EXISTS key ON connection TYPE string; DEFINE FIELD IF NOT EXISTS identityKey ON connection TYPE string; DEFINE FIELD IF NOT EXISTS data ON connection TYPE object FLEXIBLE; DEFINE INDEX IF NOT EXISTS unique_key ON connection FIELDS key UNIQUE; DEFINE INDEX IF NOT EXISTS unique_pair ON connection FIELDS identityKey UNIQUE; ");
        for (table, name, fields, unique) in [
            ("question_variant", "exact_question", "exactKey", true),
            ("question_variant", "topic_variants", "topicKey", false),
            (
                "answer_version",
                "topic_revision",
                "topicKey,revision",
                true,
            ),
            (
                "appearance",
                "exploration_members",
                "explorationKey,key",
                true,
            ),
            ("appearance", "topic_appearances", "topicKey", false),
            ("active_attempt", "one_active_topic", "topicKey", true),
            (
                "edge_support",
                "occurrence_support",
                "connectionKey,appearanceKey",
                true,
            ),
            (
                "embedding",
                "model_input",
                "modelVersion,sourceKey,inputHash",
                true,
            ),
            ("migration_manifest", "import_digest", "identityKey", true),
            ("import_receipt", "import_digest", "identityKey", true),
            (
                "migration_stage",
                "import_batch",
                "importKey,batchIndex",
                true,
            ),
            (
                "exploration",
                "ordered_explorations",
                "updatedAt,key",
                false,
            ),
        ] {
            sql.push_str(&format!(
                "DEFINE INDEX IF NOT EXISTS {name} ON {table} FIELDS {fields} {}; ",
                if unique { "UNIQUE" } else { "" }
            ));
        }
        sql.push_str("IF !record::exists(library:main) { CREATE library:main SET key=$libraryId,epoch=1,schemaVersion=$schemaVersion; }; COMMIT TRANSACTION;");
        self.query(
            &sql,
            serde_json::json!({"libraryId":LibraryId::generate(),"schemaVersion":SCHEMA_VERSION}),
        )
        .await?;
        Ok(())
    }
}
