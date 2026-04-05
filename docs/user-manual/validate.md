# git-lore validate

## Description

A CI/CD tool to run logic checks over the current workspace or branch vs the canon Lore rules.

If any local modification violates an `Accepted` decision/rule, or if there is severe structural entropy in the records, it triggers validation scripts or fails the process instantly.

## Usage

`git-lore validate [PATH]`
