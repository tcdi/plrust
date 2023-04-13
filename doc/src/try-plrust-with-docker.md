# Try PL/Rust with Docker

Giving PL/Rust a try has never been easier with Docker! This document outlines what is required to get a functional Postgres + PL/Rust environment running with just a few commands.

The PL/Rust repository contains a Dockerfile named `Dockerfile.try` that contains everything necessary to spin up and test PL/Rust in a target environment.

The following instructions assume a very basic understanding of what [Docker](https://www.docker.com) is and that it is already installed in the target environment. If Docker is not yet installed yet, instructions can be found here: <https://docs.docker.com/engine/install/>

1. Check out the PL/Rust code, and switch to that directory
1. From a command line, run the following from the root of the checkout directory (note that `sudo` or equivalent may be required):
    ```
    docker build -f Dockerfile.try -t tcdi/try-plrust .
    ```
    Note that this may take a little while to finish.
1. Once the above has finished, run the following (`sudo` may be required here):
    ```
    docker run -it tcdi/try-plrust
    ```
1. There will be some output that the Postgres server has started, and `psql` prompt will start up in the foreground:
    ```
    Type "help" for help.

    postgres(plrust)=#
    ```

That's it! From here, the `psql` interactive prompt with PL/Rust installed can be used to create and run PL/Rust functions. Here is a very small example to get started:

```SQL
CREATE FUNCTION plrust.one()
    RETURNS INT LANGUAGE plrust
AS
$$
    Ok(Some(1))
$$;
```

Creating PL/Rust functions causes Rust code compilation in the backend, so this may take some time depending on  the host's hardware specifications and internet connection speeds. Once this completes, the PL/Rust function can be executed similar to other Postgres functions:

```SQL
SELECT * FROM plrust.one();
```

which will provide the following results:

```
postgres(plrust)=# SELECT * FROM plrust.one();
 one
-----
   1
(1 row)
```

To exit out of the prompt and the Docker container, type the Postgres command `quit`:
```
postgres(plrust)=# quit
```

## Alternate running modes

Running the Docker container using `docker run -it tcdi/try-plrust` as described above will spin up both the Postgres server in the background and the `psql` command line utility in the foreground in the same running container. However, the option exists to run the Postgres server only (with PL/Rust installed) so that an alternative Postgres client can be used. To do this, run the following command (which may require `sudo` or equivalent):

```
docker run -it -p 5432:5432 tcdi/try-plrust server
```

This will set up everything that is necessary and run the Postgres server only, binding to TCP port 5432. Output here will be all of the Postgres log entries, including any errors that result from a PL/Rust compilation error. The final `server` argument in the command indicates that it should launch the `server` script upon container bootup. In order to connect with an alternative client, the only pieces of information that are required are the Postgres username (`postgres`), the hostname or IP address (e.g. `localhost` or `192.168.0.2`) and the port (`5432`). There is no password set for the `postgres` user in this setup. An example Postgres URI might look like this:

```
postgres://postgres@localhost:5432
```

To exit out of server mode, press Ctrl+c in the running Docker container.

## Caveats

* This Dockerfile and resulting image should not be used in production. It does not take many security precautions into consideration. As such, the way `Dockerfile.try` is constructed should not be considered a best practice as it relates to setting up and securing a Postgres instance with PL/Rust installed.

* The Postgres data directories, logs and built PL/Rust functions are not persistent and are destroyed upon continer termination. Externally mounting Postgres' data, log and function directories is outside the scope of this example.
