# Try PL/Rust with Docker

If you would like to give PL/Rust a try without succumbing to the long arduous process of installing everything that is required, then this next section is for you!

The PL/Rust repository contains a Dockerfile named `Dockerfile.try` that contains everything you need to spin up and test PL/Rust in your own environment.

The following instructions assume that you have a very basic understanding of what [Docker](https://www.docker.com) is and that you already have it installed. If you do not have Docker installed yet, instructions for your particular environment can be found here: <https://docs.docker.com/engine/install/>

1. Check out the PL/Rust code, and switch to that directory
1. From a command line, run the following from the root of the checkout directory (note that `sudo` or equivalent may be required):
    ```
    docker build -f Dockerfile.try -t tcdi/try-plrust .
    ```
1. Go grab your favorite beverage, because this may take some time.
1. Once that finished, run the following (`sudo` may be required here):
    ```
    docker run -it tcdi/try-plrust
    ```
1. You will see some output that the Postgres server has started, and you will be presented with a `psql` prompt:
    ```
    Type "help" for help.

    postgres(plrust)=#
    ```

That's it! From here, you can try out PL/Rust with the interactive prompt. Here is a very small example to get you started:

```SQL
CREATE FUNCTION plrust.one()
    RETURNS INT LANGUAGE plrust
AS
$$
    Ok(Some(1))
$$;
```

Remember that creating PL/Rust functions cause compilation in the backend, so this may take some time depending on your hardware specifications. Once this completes, you can execute the function just as you would with any Postgres function:

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

Running the Docker container using `docker run -it tcdi/try-plrust` as described  above will spin up both the Postgres server in the background and the `psql` command line utility in the foreground in the same running environment. However, if you'd like to use the Postgres server only (with PL/Rust installed) so that you can use your favorite Postgres client of choice, you can run the following (which may require `sudo` or equivalent):

```
docker run -it -p 5432:5432 tcdi/try-plrust server
```

This will set up everything that is necessary and run the Postgres server only, binding to TCP port 5432. The final `server` argument in the command indicates that it should launch the `server` script upon container bootup.

To exit out of server mode, press Ctrl+c.

## Caveats

* This Docker setup does not take many security precautions into consideration. As such, the way `Dockerfile.try` is constructed should not be considered a best practice as it relates to setting up and securing a Postgres instance with PL/Rust installed. The resulting container that gets built from this should not be considered secure, and thus should not be used in any production environment whatsoever.

* Mapping internal Postgres directories to the host environment through [bind mounts](https://docs.docker.com/storage/bind-mounts/) or [volumes](https://docs.docker.com/storage/volumes/) is a bit tricky since Postgres doesn't like many things owned and ran by root. Because of this, any functions and data created in a session started by `docker run ...` will be destroyed upon container termination.