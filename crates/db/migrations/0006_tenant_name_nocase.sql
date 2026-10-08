-- Tenant names are unique in any case: the tenant app logs in with the name in any case, so "Ana" and "ANA" would be
-- the same login. Fails if two names already differ only by case (check before applying).
DROP INDEX tenants_name_key;
CREATE UNIQUE INDEX tenants_name_key ON tenant (name COLLATE NOCASE);
