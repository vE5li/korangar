use hashbrown::{HashMap, HashSet};
use mlua::Lua;
use ragnarok_packets::{JobId, SkillId, SkillLevel};

use super::{HashMapExt, Library, Table};
use crate::loaders::GameFileLoader;
use crate::world::library::LuaExt;

/// Key for skill requirements.
///
/// Skill requirements depend on the job of the player, so it needs to be part
/// of the lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SkillListKey {
    /// Job id of the players current job.
    job_id: Option<JobId>,
    /// Skill id.
    skill_id: SkillId,
}

impl SkillListKey {
    /// Create a skill key without a job id.
    pub fn jobless(skill_id: SkillId) -> Self {
        Self { job_id: None, skill_id }
    }

    /// Create a skill key with a job id.
    pub fn with_job(job_id: JobId, skill_id: SkillId) -> Self {
        Self {
            job_id: Some(job_id),
            skill_id,
        }
    }

    /// Creates a second key without the job id.
    ///
    /// If the key already lacked a job id, it will be returned without
    /// modification.
    fn as_jobless(mut self) -> Self {
        self.job_id = None;
        self
    }

    /// Returns the keys job id.
    fn get_job_id(&self) -> Option<JobId> {
        self.job_id
    }
}

/// Create job specific requrement entries for the required skills of the
/// provided key.
///
/// Let's take the following skill requirements as an example:
///
/// Non-job specific requirements:
///          A
///          |
///          B
///
/// Job specific requirements:
///          C
///          |
///          B
///
/// Skill `B` requires `A` by default, but requires `C` for our example job.
/// `C` might have requriements for our example job as well, in which case it
/// already has a requirements entry and everything can be evalutated correctly.
///
/// In case that `C` *doesn't* have any requrements specific to our example job,
/// the entry for `C` will not be able to infer that it is required for `B` when
/// running the evaluation. Therefore, we create a new entry for `C`, let's call
/// it `C*`, that is job-specific and has the same requirements as `C`.
///
/// This will be done recursively for any requirements of `C*`.
fn overlay_job_entries(result: &mut HashMap<SkillListKey, SkillListRequirements>, key: SkillListKey) {
    let Some(job_id) = key.get_job_id() else {
        return;
    };

    let required_skills = result.get(&key).unwrap().required_skills.clone();

    for skill_id in required_skills.into_keys() {
        let dependency_key = SkillListKey::with_job(job_id, skill_id);

        if !result.contains_key(&dependency_key) {
            let base_key = dependency_key.as_jobless();
            let base_required_skills = result.get(&base_key).unwrap().required_skills.clone();

            result.insert(dependency_key, SkillListRequirements {
                required_skills: base_required_skills,
                required_for_skills: Default::default(),
            });

            overlay_job_entries(result, dependency_key);
        }
    }
}

/// Merge the list of skills the given key is required for.
///
/// This will be done recursively but only once per key.
///
/// This function will overflow the stack of there is a circular dependency.
fn merge_required_for(
    result: &mut HashMap<SkillListKey, SkillListRequirements>,
    fully_evaluated: &mut HashSet<SkillListKey>,
    key: SkillListKey,
) {
    if fully_evaluated.contains(&key) {
        return;
    }

    let required_for_skills = result.get(&key).unwrap().required_for_skills.clone();

    for (skill_id, skill_level) in required_for_skills.into_iter() {
        // If this entry is job specific there is a job-specific entry for the other
        // skill, use that one instead of the jobless one.
        let requiree_key = if let Some(job_id) = key.get_job_id()
            && result.contains_key(&SkillListKey::with_job(job_id, skill_id))
        {
            SkillListKey::with_job(job_id, skill_id)
        } else {
            SkillListKey::jobless(skill_id)
        };

        merge_required_for(result, fully_evaluated, requiree_key);

        let requiree_required_for_skills = result
            .get(&requiree_key)
            .unwrap()
            .required_for_skills
            .keys()
            .copied()
            .collect::<Vec<_>>();

        let entry = result.get_mut(&key).unwrap();

        requiree_required_for_skills
            .into_iter()
            .for_each(|skill_id| entry.set_required_for(skill_id, skill_level));
    }

    fully_evaluated.insert(key);
}

/// Skill requirements, including skills required for this skill to be leveled
/// and skills that require this skill to be leveled.
#[derive(Debug, Clone, Default)]
pub struct SkillListRequirements {
    /// A list of the skills directly required to level this skill.
    pub required_skills: HashMap<SkillId, SkillLevel>,
    /// A list of the all the skills that, directly and indirectly, require this
    /// skill to be leveled.
    pub required_for_skills: HashMap<SkillId, SkillLevel>,
}

impl SkillListRequirements {
    /// Set a skill as required for this skill.
    pub fn set_required(&mut self, skill_id: SkillId, skill_level: SkillLevel) {
        if let Some(previous_level) = self.required_skills.insert(skill_id, skill_level) {
            self.required_skills.get_mut(&skill_id).unwrap().0 = skill_level.0.max(previous_level.0);
        }
    }

    /// Set this skill as required for another skill.
    pub fn set_required_for(&mut self, skill_id: SkillId, skill_level: SkillLevel) {
        if let Some(previous_level) = self.required_for_skills.insert(skill_id, skill_level) {
            self.required_for_skills.get_mut(&skill_id).unwrap().0 = skill_level.0.max(previous_level.0);
        }
    }
}

impl Table for SkillListRequirements {
    type Key<'a> = SkillListKey;
    type Storage = HashMap<SkillListKey, Self>;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        let state = Lua::load_from_game_files(game_file_loader, &[
            // Needed to get the `JOBID` table.
            "data\\luafiles514\\lua files\\skillinfoz\\jobinheritlist.lub",
            // Needed to get the `SKID` table.
            "data\\luafiles514\\lua files\\skillinfoz\\skillid.lub",
            "data\\luafiles514\\lua files\\skillinfoz\\skillinfolist.lub",
        ])?;

        let globals = state.globals();
        let skill_requirements_list = globals.get::<mlua::Table>("SKILL_INFO_LIST")?;

        // To describe the algorithm we use to compute the requirements entries, the
        // documentation will refer to the following (made up) skills entries from Lua:
        //
        // [SKID.SU_BASIC_SKILL] = {
        // }
        //
        // [SKID.RL_C_MARKER] = {
        //     _NeedSkillList = {
        //         { SKID.SU_BASIC_SKILL, 3 }
        //     }
        // }
        //
        // [SKID.AL_HEAL] = {
        //     NeedSkillList = {
        //         [JOBID.JT_CRUSADER] = {
        //             { SKID.SU_BASIC_SKILL, 10 },
        //         }
        //     }
        // }
        //
        // [SKID.KN_BOWLINGBASH] = {
        //     _NeedSkillList = {
        //         { SKID.RL_C_MARKER, 10 },
        //     },
        //     NeedSkillList = {
        //         [JOBID.JT_SUPERNOVICE2] = {
        //             { SKID.SU_BASIC_SKILL, 5 }
        //         }
        //     }
        // }

        let mut result: Self::Storage = HashMap::new();

        // Step 1: we create entries for every skill and their direct dependencies.
        //
        // After this step, the entries look like this:
        //
        // ```
        // None                        + SKID.SU_BASIC_SKILL - { required_skills: [],                            required_for_skills: [] }
        // None                        + SKID.RL_C_MARKER    - { required_skills: [{ SKID.SU_BASIC_SKILL, 3 }],  required_for_skills: [] }
        // Some(JOBID.JT_CRUSADER)     + SKID.AL_HEAL        - { required_skills: [{ SKID.SU_BASIC_SKILL, 10 }], required_for_skills: [] }
        // None                        + SKID.KN_BOWLINGBASH - { required_skills: [{ SKID.RL_C_MARKER, 10 }],    required_for_skills: [] }
        // Some(JOBID.JT_SUPERNOVICE2) + SKID.KN_BOWLINGBASH - { required_skills: [{ SKID.SU_BASIC_SKILL, 5 }],  required_for_skills: [] }
        // ```
        for (skill_id, table) in skill_requirements_list.pairs::<u16, mlua::Table>().flatten() {
            result.insert(SkillListKey::jobless(SkillId(skill_id)), Default::default());

            if let Ok(sequence) = table.get::<mlua::Table>("_NeedSkillList") {
                for required_skill in sequence.sequence_values::<mlua::Table>() {
                    let required_skill = required_skill?;
                    let required_skill_id = required_skill.get::<u16>(1)?;
                    // Some skills don't have a level set, e.g. `WS_CARTBOOST`, which I believe
                    // means that they only need to be unlocked.
                    let skill_level = required_skill.get::<u16>(2).unwrap_or(1);

                    result
                        .get_mut(&SkillListKey::jobless(SkillId(skill_id)))
                        .unwrap()
                        .set_required(SkillId(required_skill_id), SkillLevel(skill_level));
                }
            }

            if let Ok(table) = table.get::<mlua::Table>("NeedSkillList") {
                for (job_id, sequence) in table.pairs::<u16, mlua::Table>().flatten() {
                    for required_skill in sequence.sequence_values::<mlua::Table>() {
                        let required_skill = required_skill?;
                        let required_skill_id = required_skill.get::<u16>(1)?;
                        // Some skills don't have a level set, e.g. `WS_CARTBOOST`, which I believe
                        // means that they only need to be unlocked.
                        let skill_level = required_skill.get::<u16>(2).unwrap_or(1);

                        result
                            .entry(SkillListKey::with_job(JobId(job_id), SkillId(skill_id)))
                            .or_default()
                            .set_required(SkillId(required_skill_id), SkillLevel(skill_level));
                    }
                }
            }
        }

        // Step 2: create missing job specific entries.
        //
        // After this step, the entries look like this:
        //
        // ```
        // None                        + SKID.SU_BASIC_SKILL - { required_skills: [],                            required_for_skills: [] }
        // None                        + SKID.RL_C_MARKER    - { required_skills: [{ SKID.SU_BASIC_SKILL, 3 }],  required_for_skills: [] }
        // Some(JOBID.JT_CRUSADER)     + SKID.AL_HEAL        - { required_skills: [{ SKID.SU_BASIC_SKILL, 10 }], required_for_skills: [] }
        // None                        + SKID.KN_BOWLINGBASH - { required_skills: [{ SKID.RL_C_MARKER, 10 }],    required_for_skills: [] }
        // Some(JOBID.JT_SUPERNOVICE2) + SKID.KN_BOWLINGBASH - { required_skills: [{ SKID.SU_BASIC_SKILL, 5 }],  required_for_skills: [] }
        // -- Newly added entries
        // Some(JOBID.JT_CRUSADER)     + SKID.SU_BASIC_SKILL - { required_skills: [],                            required_for_skills: [] }
        // Some(JOBID.JT_SUPERNOVICE2) + SKID.SU_BASIC_SKILL - { required_skills: [],                            required_for_skills: [] }
        // ```
        for key in result.keys().cloned().collect::<Vec<_>>() {
            overlay_job_entries(&mut result, key);
        }

        // Step 4: Fill the `required_for_skills` field of every entry with its *direct*
        // dependencies.
        //
        // After this step, the entries look like this:
        //
        // ```
        // None                        + SKID.SU_BASIC_SKILL - { required_skills: [],                            required_for_skills: [{ SKID.RL_C_MARKER, 3 }] }
        // None                        + SKID.RL_C_MARKER    - { required_skills: [{ SKID.SU_BASIC_SKILL, 3 }],  required_for_skills: [{ SKID.KN_BOWLINGBASH, 10 }] }
        // Some(JOBID.JT_CRUSADER)     + SKID.AL_HEAL        - { required_skills: [{ SKID.SU_BASIC_SKILL, 10 }], required_for_skills: [] }
        // None                        + SKID.KN_BOWLINGBASH - { required_skills: [{ SKID.RL_C_MARKER, 10 }],    required_for_skills: [] }
        // Some(JOBID.JT_SUPERNOVICE2) + SKID.KN_BOWLINGBASH - { required_skills: [{ SKID.SU_BASIC_SKILL, 5 }],  required_for_skills: [] }
        // -- Newly added entries
        // Some(JOBID.JT_CRUSADER)     + SKID.SU_BASIC_SKILL - { required_skills: [],                            required_for_skills: [{ SKID.AL_HEAL, 10 }] }
        // Some(JOBID.JT_SUPERNOVICE2) + SKID.SU_BASIC_SKILL - { required_skills: [],                            required_for_skills: [{ SKID.KN_BOWLINGBASH, 5 }] }
        // ```
        {
            let updates = result
                .keys()
                .map(|key| {
                    let job_id = key.job_id;
                    let skill_id = key.skill_id;

                    let required_for = result
                        .iter()
                        .filter(|(key, _)| {
                            // Only check entries that either:
                            // - Have the same job as the one we want to update
                            // - Have _no_ job and also no entry for this job
                            key.job_id == job_id
                                || (key.job_id.is_none()
                                    && !result.contains_key(&SkillListKey {
                                        job_id,
                                        skill_id: key.skill_id,
                                    }))
                        })
                        .filter_map(|(key, value)| value.required_skills.get(&skill_id).map(|skill_level| (key.skill_id, *skill_level)))
                        .collect::<Vec<_>>();

                    (*key, required_for)
                })
                .collect::<Vec<_>>();

            for (key, required_for_skills) in updates {
                let entry = result.get_mut(&key).unwrap();

                for (skill_id, skill_level) in required_for_skills {
                    entry.set_required_for(skill_id, skill_level);
                }
            }
        }

        // Step 4: Merge the `required_for_skills` lists of all entries.
        //
        // After this step, the entries look like this:
        //
        // ```
        // None                        + SKID.SU_BASIC_SKILL - { required_skills: [],                            required_for_skills: [{ SKID.RL_C_MARKER, 3 }, { SKID.KN_BOWLINGBASH, 3 }] }
        // None                        + SKID.RL_C_MARKER    - { required_skills: [{ SKID.SU_BASIC_SKILL, 3 }],  required_for_skills: [{ SKID.KN_BOWLINGBASH, 10 }] }
        // Some(JOBID.JT_CRUSADER)     + SKID.AL_HEAL        - { required_skills: [{ SKID.SU_BASIC_SKILL, 10 }], required_for_skills: [] }
        // None                        + SKID.KN_BOWLINGBASH - { required_skills: [{ SKID.RL_C_MARKER, 10 }],    required_for_skills: [] }
        // Some(JOBID.JT_SUPERNOVICE2) + SKID.KN_BOWLINGBASH - { required_skills: [{ SKID.SU_BASIC_SKILL, 5 }],  required_for_skills: [] }
        // -- Newly added entries
        // Some(JOBID.JT_CRUSADER)     + SKID.SU_BASIC_SKILL - { required_skills: [],                            required_for_skills: [{ SKID.AL_HEAL, 10 }] }
        // Some(JOBID.JT_SUPERNOVICE2) + SKID.SU_BASIC_SKILL - { required_skills: [],                            required_for_skills: [{ SKID.KN_BOWLINGBASH, 5 }] }
        // ```
        for key in result.keys().cloned().collect::<Vec<_>>() {
            merge_required_for(&mut result, &mut HashSet::new(), key);
        }

        Ok(result.compact())
    }

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self>
    where
        Self: Sized,
    {
        library
            .skill_requirements_table
            .get(&key)
            .or_else(|| library.skill_requirements_table.get(&key.as_jobless()))
    }

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self
    where
        Self: Sized,
    {
        Self::try_get(library, key).expect("incomplete skill tree")
    }
}
