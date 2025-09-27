use anyhow::{Error as E, Result};
use sqlx::{Pool, Postgres, migrate::Migrator};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PostgresStorageGateway {
    pool: Pool<Postgres>,
}

impl PostgresStorageGateway {
    #[inline(always)]
    pub async fn new(connection_string: &str) -> Result<Self> {
        let pool = Pool::connect(connection_string).await.map_err(E::msg)?;
        Ok(Self { pool })
    }

    #[inline(always)]
    pub async fn migrate(&self, migrator: Migrator) -> Result<()> {
        migrator.run(self.get_pool()).await.map_err(E::msg)
    }

    #[inline(always)]
    pub fn get_pool(&self) -> &Pool<Postgres> {
        &self.pool
    }
}

/// Represents a type that can insert entities in bulk into storage.
#[async_trait::async_trait]
pub trait StoreInsertBulk<Entity, Identifier> {
    /// Inserts multiple entities into storage.
    ///
    /// # Arguments
    ///
    /// * `entities` - Slice of entities to insert.
    ///
    /// # Returns
    ///
    /// * Returns a vector of unique identifiers of the inserted entities on success, or an error otherwise.
    async fn insert_bulk(&self, entities: &[Entity]) -> Result<Vec<Identifier>>;
}

/// Represents a type that can read multiple entities by their IDs from storage.
#[async_trait::async_trait]
pub trait StoreReadBulkEntities<Entity, Identifier> {
    /// Reads multiple entities by their identifiers.
    ///
    /// # Arguments
    ///
    /// * `ids` - Slice of identifiers.
    ///
    /// # Returns
    ///
    /// * Returns a vector of entities on success, or an error otherwise.
    async fn read_bulk_by_ids(&self, ids: &[Identifier]) -> Result<Vec<Entity>>;
}

/// Represents a type that can filter and paginate entities from storage.
#[async_trait::async_trait]
pub trait StorePaginateBulkEntities<Entity> {
    /// Filters and paginates entities from storage.
    ///
    /// # Arguments
    ///
    /// * `field_map` - Map of field names and filter values.
    /// * `limit` - Number of entities per page.
    /// * `offset` - Offset to start pagination.
    ///
    /// # Returns
    ///
    /// * Returns a vector of entities on success, or an error otherwise.
    async fn filter_paginate(
        &self,
        field_map: &HashMap<String, String>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Entity>>;
}

#[macro_export]
macro_rules! count_exprs {
    () => (0usize);
    ($head:expr) => (1usize);
    ($head:expr, $($tail:expr),*) => (1usize + crate::count_exprs!($($tail),*));
}

#[macro_export]
macro_rules! impl_store_bulk {
    (
        $model:ty, $id_type:ty, $table_name:literal,
        [$($field:ident),+ $(,)?],
        $conflict_field:literal,
    ) => {
        #[async_trait::async_trait]
        impl crate::database::StoreInsertBulk<$model, $id_type> for crate::database::PostgresStorageGateway {
            #[inline(always)]
            async fn insert_bulk(&self, transactions: &[$model]) -> Result<Vec<$id_type>> {
                if transactions.is_empty() {
                    return Err(anyhow!("Found zero items to insert into `{}`.", $table_name));
                }

                let mut query = format!(
                    "INSERT INTO {} ({}) VALUES",
                    $table_name,
                    stringify!($($field),*).replace(" ", "")
                );

                let mut params: Vec<String> = Vec::new();
                let field_count = crate::count_exprs!($($field),*);
                for i in 0..transactions.len() {
                    let placeholders: Vec<String> = (1..=field_count)
                        .map(|j| format!("${}", i * field_count + j))
                        .collect();
                    params.push(format!("({})", placeholders.join(", ")));
                }

                query.push_str(&params.join(", "));
                query.push_str(&format!(" ON CONFLICT ({}) DO UPDATE SET ", $conflict_field));

                let mut update_assignments = vec![];
                $(
                    if stringify!($field) != $conflict_field {
                        update_assignments.push(format!("{} = EXCLUDED.{}", stringify!($field), stringify!($field)));
                    }
                )+

                query.push_str(&update_assignments.join(", "));
                query.push_str(&format!(" RETURNING {}", $conflict_field));

                let mut query_builder = sqlx::query(&query);
                for entity in transactions.iter() {
                    $(
                        query_builder = query_builder.bind(entity.$field.clone());
                    )+
                }

                let mut tx = self.get_pool().begin().await?;
                let rows = query_builder.fetch_all(&mut *tx).await?;
                let ids: Vec<$id_type> = rows.into_iter().map(|row| row.get($conflict_field)).collect();
                tx.commit().await?;

                Ok(ids)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_read_bulk_by_ids {
    (
        $model:ty, $id_type:ty,
        $table_name:literal,
        [$($field:ident),+ $(,)?],
        $id_field:literal,
    ) => {
        #[async_trait::async_trait]
        impl StoreReadBulkEntities<$model, $id_type> for crate::PostgresStorageGateway {
            #[inline(always)]
            async fn read_bulk_by_ids(&self, ids: &[$id_type]) -> Result<Vec<$model>> {
                if ids.is_empty() {
                    return Err(anyhow!("Found zero identifiers to read from `{}`.", $table_name));
                }

                let fields = vec![$(stringify!($field)),+].join(", ");
                let placeholders: Vec<String> = (1..=ids.len())
                    .map(|i| format!("${}", i))
                    .collect();
                let query_str = format!(
                    "SELECT {} FROM {} WHERE {} IN ({})",
                    fields,
                    $table_name,
                    $id_field,
                    placeholders.join(", ")
                );

                let mut args = PgArguments::default();
                for id in ids {
                    let _ = args.add(id);
                }

                let rows = sqlx::query_as_with::<_, $model, _>(&query_str, args)
                    .fetch_all(self.get_pool())
                    .await?;

                Ok(rows)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_read_bulk_multiple {
    (
        $model:ty,
        $table_name:literal,
        [$($field:ident),+ $(,)?],
        $field_map_type:ty
    ) => {
        #[async_trait::async_trait]
        impl crate::database::StorePaginateBulkEntities<$model> for crate::PostgresStorageGateway {
            #[inline(always)]
            async fn filter_paginate(
                &self,
                field_map: $field_map_type,
                limit: i64,
                offset: i64,
            ) -> Result<Vec<$model>> {
                let valid_fields: Vec<_> = field_map
                    .iter()
                    .filter(|(k, v)| !k.trim().is_empty() && !v.trim().is_empty())
                    .collect();

                if valid_fields.is_empty() {
                    return Err(anyhow!("No valid filters found for `{}`.", $table_name));
                }

                let fields = vec![$(stringify!($field)),+].join(", ");
                let filters = valid_fields
                    .iter().enumerate()
                    .map(|(i, (field_name, _))| format!("{} = ${}", field_name, i + 1))
                    .collect::<Vec<_>>()
                    .join(" AND ");
                let query_str = format!(
                    "SELECT {} FROM {} WHERE {} LIMIT {} OFFSET {}",
                    fields, $table_name, filters, limit, offset
                );

                let mut args = PgArguments::default();
                for (_, value) in valid_fields {
                    let _ = args.add(value);
                }

                let rows = sqlx::query_as_with::<_, $model, _>(&query_str, args)
                    .fetch_all(self.get_pool())
                    .await?;

                Ok(rows)
            }
        }
    };
}

#[macro_export]
macro_rules! read_all_last {
    (
        $model:ty, $table_name:literal,
        [$($field:ident),+ $(,)?],
    ) => {
        #[async_trait::async_trait]
        impl StoreReadAll<$model> for PostgresStorageGateway {

            async fn read_all(&self) -> Result<Vec<$model>> {
                let fields = vec![$(stringify!($field)),+].join(", ");
                let query_str = format!("SELECT {} FROM {}", fields, $table_name);

                let rows = sqlx::query_as::<_, $model>(&query_str)
                    .fetch_all(self.get_pool())
                    .await?;
                Ok(rows)
            }
        }
    };
}
