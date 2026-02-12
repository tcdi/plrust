/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/
DROP FUNCTION IF EXISTS plrust.list_allowed_dependencies();

-- plrust/src/lib.rs:197
-- plrust::list_allowed_dependencies
CREATE FUNCTION plrust."allowed_dependencies"() RETURNS TABLE (
	"name" TEXT,  /* alloc::string::String */
	"version" TEXT,  /* alloc::string::String */
	"features" TEXT[],  /* alloc::vec::Vec<alloc::string::String> */
	"default_features" bool  /* bool */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'allowed_dependencies_wrapper';
