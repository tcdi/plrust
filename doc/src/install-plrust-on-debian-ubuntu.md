# Install PL/Rust on Debian/Ubuntu

Debian packages for PL/Rust are available for download on the Github Releases page here: <https://github.com/tcdi/plrust/releases>

## Assumptions

The provided Debian packages for PL/Rust make certain assumptions about the environment in which they are installed. Notably, they require PostgreSQL either has been installed or will be installed with the official [Postgres Debian packages](https://www.postgresql.org/download/linux/debian/). Because of this, the PL/Rust Debian packages assume:

* The user which runs Postgres is `postgres`
* The home directory for the `postgres` user is located at `/var/lib/postgresql`
* The `postgres` user has the ability to create and manage databases and extensions
* The `postgres` user has the ability to install Rust and all of the required dependencies

## Filename convention

The PL/Rust artifacts that are uploaded to Github releases follow this pattern:

```
plrust-trusted-<PLRUSTVER>_<RUSTTOOLCHAINVER>-debian-pg<PGMAJORVER>-<DPKGARCH>.deb
```

Where:
* PLRUSTVER is the PL/Rust release version
* RUSTTOOLCHAINVER is the version of Rust+toolchains in which the Debian package was built
* PGMAJORVER is the major version of PostgreSQL the package targets, such as 13, 14 or 15
* DPKGARCH is the CPU architecture name according to `dpkg`, such as `arm64` or `amd64`

Example:

```
plrust-trusted-1.2.3_1.67.1-debian-pg15-amd64.deb
```

## Preparing the environment

Certain applications, libraries and dependencies must be set up before PL/Rust can be installed from a Debian package.

Note that `sudo` may be required during the setup and configuration of certain system components.


### System and development requirements

Because PL/Rust is a compiled language, certain libraries and development tools will be required to be installed:

```
apt-get update && \
apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    clang \
    clang-11 \
    gcc \
    git \
    gnupg \
    libssl-dev \
    llvm-11 \
    lsb-release \
    make \
    pkg-config \
    wget
```

### Installing Postgres

If Postgres has already been installed with the official [Postgres Debian packages](https://www.postgresql.org/download/linux/debian/), then skip this section and proceed to [Installing Rust and components](#installing-rust-and-components)

1. Set up the official PostgreSQL APT repository:
    ```
    echo "deb http://apt.postgresql.org/pub/repos/apt $(lsb_release -cs)-pgdg main" > \
      /etc/apt/sources.list.d/pgdg.list
    ```
    ```
    wget --quiet -O - https://www.postgresql.org/media/keys/ACCC4CF8.asc | \
      gpg --dearmor | tee /etc/apt/trusted.gpg.d/apt.postgresql.org.gpg >/dev/null
    ```
1. Update APT and install PostgreSQL. Replace `XX` with the PostgreSQL major version to be installed (e.g. 13, 14, 15):
    ```
    apt-get update -y -qq --fix-missing && \
    apt-get install -y --no-install-recommends \
        postgresql-XX \
        postgresql-server-dev-XX
    ```

### Installing Rust and components

A key component of making PL/Rust work is the Rust compiler and components. However, Rust and its tooling must be installed as the `postgres` user. If the package-required version of Rust and toolchains are already installed and defaulted for the `postgres` user, then skip this section and proceed to [Installing PL/Rust](#installing-plrust).

Instructions that follow assume the latest version of PL/Rust, which requires `{{toolchain_ver}}` of Rust and toolchain set. If the desired PL/Rust Debian package requires a different version of the toolchain set (as indicated by the [filename](#filename-convention)), then substitute that version in the following instructions.

1. Switch to the `postgres` user:
    ```
    su -l - postgres
    ```
1. If Rust has never been installed as the `postgres` user, then run the following:
    ```
    wget -qO- https://sh.rustup.rs | \
      sh -s -- \
      -y \
      --profile minimal \
      --default-toolchain={{toolchain_ver}}
    ```
1. If Rust has previously been installed as the `postgres` user, then ensure that the `{{toolchain_ver}}` toolchain is installed and set to default:
    ```
    rustup toolchain install {{toolchain_ver}}
    rustup default {{toolchain_ver}}
    ```
1. Ensure that the `rustc-dev` component has been installed:
    ```
    rustup component add rustc-dev
    ```

Future versions of PL/Rust may require a different version of the Rust toolchain to be installed and set to default. In such an event, step 3 and onward must be repeated with the new required version of the specific toolchain. The filename of the PL/Rust Debian package contains the version of the Rust toolchain it was built with -- see [Filename convention](#filename-convention) for more details.

### Installing PL/Rust

With the prerequisites installed and set up, it is time to install the PL/Rust Debian package:

1. Head to [the PL/Rust releases page](https://github.com/tcdi/plrust/releases) and download the appropriate version onto the target system
1. Install the package:
    ```
    apt install /path/to/plrust-trusted-X.X.X_{{toolchain_ver}}-debian-pgXX-yourarch.deb
    ```

The package installation will fail if at least one of the above Rust dependencies are not met.

#### Service Restart Notice

Newer versions of Debian/Ubuntu may prompt for a restart of certain services during the Debian package installation process, notably the PostgreSQL server service. Take caution in determining the appropriate time for the PostgreSQL service to be restarted. For example, if the GUC settings have not been set up before as outlined in the following [Configuring PostgreSQL](#configuring-postgresql) section, then it may be advisable to delay the restart until after those are set up. It may also be advisable to delay a restart for a system that is already in production.

Regardless of timing, PostgreSQL will need to be restarted in any of the following conditions:
1. PL/Rust is installed for the first time and GUC additions have been added
1. PL/Rust is updated on an existing system at some point in the future, with or without GUC changes
1. Any GUC change is required for an existing PL/Rust setup

### Configuring PostgreSQL

In order for PL/Rust to be available to the PostgreSQL server, some new [GUC](https://www.postgresql.org/docs/current/config-setting.html) settings must configured. See the [PostgreSQL Configuration for PL/Rust](config-pg.md) for more details on the required setup and other options that may be necessary.

Any configuration changes will require a restart of the PostgreSQL service on the system.

### Finishing up

To test if PL/Rust is set up correctly, load up `psql` as the `postgres` user and run the following:

```SQL
CREATE EXTENSION plrust;
```

Then, create a simple function and try it out:

```SQL
CREATE OR REPLACE FUNCTION plrust.one()
    RETURNS INT LANGUAGE plrust
AS
$$
    Ok(Some(1))
$$;
```

```SQL
SELECT plrust.one();
```

```
┌─────┐
│ one │
╞═════╡
│   1 │
└─────┘
```

## Updating

Future versions of PL/Rust can be installed using the same methods described in [Installing PL/Rust](#installing-plrust). One consideration when upgrading is that new versions of PL/Rust may require a different version of the Rust toolchain. See [Installing Rust and components](#installing-rust-and-components) for details.