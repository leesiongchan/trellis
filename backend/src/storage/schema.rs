//! Versioned schema. Indexed identity fields stay outside flexible domain payloads.
use super::*;
use crate::contracts::library::LibraryId;
pub(crate) const SCHEMA_VERSION: u64 = 1;
impl KnowledgeStore {
    pub async fn install_schema(&self) -> Result<(), AppError> {
        let mut sql = String::from(
            "BEGIN TRANSACTION; DEFINE TABLE IF NOT EXISTS library SCHEMAFULL; DEFINE FIELD IF NOT EXISTS key ON library TYPE string; DEFINE FIELD IF NOT EXISTS epoch ON library TYPE int; DEFINE FIELD IF NOT EXISTS schemaVersion ON library TYPE int; ",
        );
        sql.push_str("DEFINE FIELD IF NOT EXISTS importSerial ON library TYPE option<int>; DEFINE FIELD IF NOT EXISTS discoveryStatus ON library TYPE option<string>; DEFINE FIELD IF NOT EXISTS activeModelVersion ON library TYPE option<string>; ");
        let tables: &[(&str, &[(&str, &str)])] = &[
            (
                "verification_stage",
                &[("key", "string"), ("attemptKey", "string")],
            ),
            ("match_state", &[("key", "string")]),
            ("embedding_model", &[("key", "string")]),
            ("embedding_cursor", &[("key", "string")]),
            (
                "embedding_state",
                &[
                    ("key", "string"),
                    ("modelVersion", "string"),
                    ("sourceKey", "string"),
                    ("inputHash", "string"),
                ],
            ),
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
                    ("pinRevision", "option<int>"),
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
                    ("attemptKey", "option<string>"),
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
                "research_follower",
                &[
                    ("key", "string"),
                    ("attemptKey", "string"),
                    ("appearanceKey", "string"),
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
                    ("partIndex", "int"),
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
        sql.push_str("REMOVE INDEX IF EXISTS model_input ON embedding; ");
        for (table, name, fields, unique) in [
            ("embedding", "model_scan", "modelVersion,key", false),
            (
                "embedding_state",
                "source_generation",
                "modelVersion,sourceKey,inputHash",
                true,
            ),
            ("appearance", "pinned_versions", "answerVersionKey", false),
            (
                "research_follower",
                "attempt_followers",
                "attemptKey,key",
                false,
            ),
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
                "model_input_part",
                "modelVersion,sourceKey,inputHash,partIndex",
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
