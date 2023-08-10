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
