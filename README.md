# IssueCraft

A new take on issue tracking.

## **ATTENTION**

## Overview

IssueCraft is an issue tracker that's meant to be production ready, yet simple to use and extend.

## Features

- Create and manage projects, issues, and users
- Custom query language (IQL) for interacting with the system

## Installation

```sh
cargo install --path .
```

## Usage

Run queries using the cli:

```sh
issuecraft "CREATE PROJECT myproject"
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
