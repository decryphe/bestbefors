#!/bin/bash -ex

# translation (lookup)
#
# Contains all translation entries for a multilingual UI. This will contain both
# static and dynamic entries.
#
cargo loco generate model translation \
  code:string! \
  lang:string! \
  text:string!

# expiry (lookup)
#
# Expiry always assume "start of day" for offset calculation.
#
cargo loco generate model expiry \
  code:string! \
  sqlite_modifier:string! \
  sqlite_num_of_modifier:int!

# interval (lookup)
#
# Intervals always assume "start of day" for offset calculation.
#
cargo loco generate model interval \
  code:string! \
  sqlite_modifier:string! \
  sqlite_num_of_modifier:int!

# result (lookup)
cargo loco generate model result \
  code:string!

# user
cargo loco generate model user \
  name:string!

# checklist
cargo loco generate model checklist \
  name:string! \
  description:text

# checklist_step
cargo loco generate model checklist_step \
  checklist:references \
  position:int! \
  name:string! \
  description:text

# Duplicating the checklist tables for "executed" checklists makes it possible
# for the user to easily go and modify the existing checklists without damaging
# history of already performed checklists.
#
# This functionality will make application design easier, separating config from
# data.

# executed_checklist
cargo loco generate model executed_checklist \
  name:string! \
  description:text

# executed_checklist_step
cargo loco generate model executed_checklist_step \
  executed_checklist:references \
  position:int! \
  name:string! \
  description:text

# inventory_item_kind
cargo loco generate model inventory_item_kind \
  checklist:references:default_checklist_id \
  interval:references:default_interval_id \
  expiry:references:default_expiry_id \
  name:string^

# inventory_item
cargo loco generate model inventory_item \
  inventory_item_kind:references \
  checklist:references \
  interval:references \
  name:string! \
  serial_number:string \
  expiry:tstz \
  last_checked_at:tstz

# inventory_item_check
cargo loco generate model inventory_item_check \
  inventory_item:references \
  executed_checklist:references \
  user:references:checked_by \
  result:references \
  finished:bool! \
  checked_at:tstz! \
  notes:text

# inventory_item_check_step
cargo loco generate model inventory_item_check_step \
  inventory_item_check:references \
  executed_checklist_step:references \
  result:references \
  notes:text
