# User Defined Types

PL/Rust supports using User Defined Types (UDTs; sometimes referred to as "composite types") in `LANGUAGE plrust` functions.
UDTs can be used as arguments and return types.

The general approach with UDTs is to first define one in SQL:

```sql
CREATE TYPE person AS (
    name text,
    age  float8
);
```

`person` can now be used in any PL/Rust function.  To instantiate a new `person`:

```sql
create function make_person(name text, age float8) returns person
    strict parallel safe
    language plrust as
$$
    // create the Heap Tuple representation of the SQL type `person`
    let mut p = PgHeapTuple::new_composite_type("person")?;
    
    // set a few of its attributes
    //
    // Runtime errors can occur if the attribute name is invalid or if the Rust type of the value
    // is not compatible with the backing SQL type for that attribute.  Hence the use of the `?` operator
    p.set_by_name("name", name)?;
    p.set_by_name("age", age)?;
    
    // return the `person`
    Ok(Some(p))
$$;
```

Individual field accessors for the properties are straight-forward:

```sql
create function get_person_name(p person) returns text
    strict parallel safe
    language plrust as
$$
   // `p` is a `PgHeapTuple` over the underlying data for `person`
   Ok(p.get_by_name("name")?)
$$;

create function get_person_age(p person) returns float8
    strict parallel safe
    language plrust as
$$
   // `p` is a `PgHeapTuple` over the underlying data for `person`
   Ok(p.get_by_name("age")?)
$$;
```

A generic accessor, for example requires encoding knowledge of the UDT structure, but provides quite a bit of flexibility.  

Note that this function `returns text`.  This is a common denominator type to represent the various attribute types used 
by `person`.  Fortunately, Postgres and PL/Rust have fantastic support for converting values to text/Strings:

```sql
create function get_person_attribute(p person, attname text) returns text
    strict parallel safe
    language plrust as
$$
   match attname.to_lowercase().as_str() {
    "age" => {
        let age:Option<f64> = p.get_by_name("age")?;
        Ok(age.map(|v| v.to_string()))
    },
    "name" => {
        Ok(p.get_by_name("name")?)
    },
    _ => panic!("unknown attribute: `{attname}`")
   }
$$;
```

This lends itself nicely to creating a custom operator to extract a `person`'s named attribute.

```sql
create operator ->> (function = get_person_attribute, leftarg = person, rightarg = text);
```

Tying these pieces together:

```sql

-- assume all of the above sql has been executed

create table people
(
    id serial8 not null primary key,
    p  person
);

insert into people (p) values (make_person('Johnny', 46.24));
insert into people (p) values (make_person('Joe', 99.09));
insert into people (p) values (make_person('Dr. Beverly Crusher of the Starship Enterprise', 32.0));

select p ->> 'name' as name, (p ->> 'age')::float8 as age from people;
                      name                      |  age  
------------------------------------------------+-------
 Johnny                                         | 46.24
 Joe                                            | 99.09
 Dr. Beverly Crusher of the Starship Enterprise |    32
(3 rows)
```

## Discussion

In Rust, [`PgHeapTuple`](https://docs.rs/plrust-trusted-pgrx/latest/plrust_trusted_pgrx/heap_tuple/struct.PgHeapTuple.html) 
is the type that generically represents all UDTs.

`PgHeapTuple` provides the ability to construct a new UDT by its SQL name.  It also provides attribute getter and setter methods
for reading and mutating attributes.  

Attributes can be addressed by name or one-based index.  Typical errors such as specifying an attribute name that doesn't 
exist, an index that is out of bounds, or a Rust type for the value that is not compatible with that attribute's SQL type 
will return a [`TryFromDatumError`](https://docs.rs/plrust-trusted-pgrx/latest/plrust_trusted_pgrx/heap_tuple/enum.TryFromDatumError.html).
An early-return that error using the `?` operator (as demonstrated in the examples above) or matching on the error are
both fine ways of handling such errors.