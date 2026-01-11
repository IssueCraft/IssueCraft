# IssueCraft

A new take on issue tracking.

**ATTENTION: IssueCraft is still in it's infancy and should be considered pre-alpha software**

This is also why the repository is private for now, until the project is a bit further along.

- [Overview](#overview)
- [Features](#features)
- [Planned Features](#planned-features)
- [Installation](#installation)
- [Usage](#usage)
- [Query Language](#query-language)

## Overview

IssueCraft will be an issue tracker that's meant to be production ready, yet simple to use and extend.

## Features

- Create and manage projects, issues, and users
- Custom query language (IQL) for interacting with the system

## Planned Features

- A server for multi-user environments

## Installation

```sh
cargo install issuecraft
```

## Usage

Run queries using the cli:

```sh
issuecraft query "CREATE PROJECT myproject"
```

## Query Language

The IssueCraft cli uses IQL (IssueCraft Query Language) as an interface between you
and the system.

Examples:

- `CREATE PROJECT myproject`
- `CREATE ISSUE IN myproject TITLE "Bug fix"`
- `SELECT * FROM issues WHERE status = open`
- `ASSIGN project#123 TO username`
- `CLOSE project#123`

More info can be found here: [issuecraft-ql](https://crates.io/crates/issuecraft-ql)
