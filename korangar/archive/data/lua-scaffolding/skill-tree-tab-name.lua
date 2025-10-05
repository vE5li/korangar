-- More optimized version of `JobSkillTab.ChangeSkillTabName` defined in `skillinfo_f.lub`.
-- Instead of storing the names as part of a sequence and iterating to get the correct entry, we use a table for the lookup.
--
-- It also handles any number of arguments, unlike the original function.
-- This allows easily adding a new tab without changing the ChangeSkillTabName function or Korangar.
-- Adding a new tab only requires adding an entry to `TAB_BASE_NAMES`.

JobSkillTab = {}

function JobSkillTab.ChangeSkillTabName(job_id, ...)
    JobSkillTab[job_id] = arg
end

-- Name for the first tab. This cannot be overridden by the ChangeSkillTabName function.
local NOVICE_TAB_NAME = "Novice"
-- Default names for the skill tree tabs. Defining them in Lua means that they will not change with the client language, but
-- given that the overriden names won't change either, I think this is the desired behavior.
--
-- NOTE: The override from ChangeSkillTabName will only apply if a respective base name exists. Meaning you will have to add
-- another entry to this table to make an additional tab work.
local TAB_BASE_NAMES = { "First", "Second", "Third", "Fourth" }

function GET_SKILL_TAB_NAME(job_id)
    -- Combined uses zero-based indexing to make Rust-interop easier.
    local combined = { [0] = NOVICE_TAB_NAME }
    local overlay = JobSkillTab[job_id] or {}

    for tab_index, name in ipairs(TAB_BASE_NAMES) do
        combined[tab_index] = overlay[tab_index] or name
    end

    return combined
end
