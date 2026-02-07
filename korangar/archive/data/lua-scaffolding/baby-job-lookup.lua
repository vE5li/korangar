-- Function to determine if a job id corresponds to a baby job.
--
-- As far as I can tell, this is hard coded in the original client. Having it
-- in Lua allows modifying/extending this easily without editing the Korangar
-- source code.

function IS_BABY_JOB(job_id)
    -- Helper function to handle nil job ids by returning -1 as a fallback.
    -- This ensures comparisons don't fail when a job id doesn't exist in `JTtbl`.
    local non_nil = function(job_id)
        return job_id or -1
    end

    return (job_id >= non_nil(JTtbl.JT_NOVICE_B) and job_id <= non_nil(JTtbl.JT_SUPERNOVICE_B)) or
        (job_id >= non_nil(JTtbl.JT_RUNE_KNIGHT_B) and job_id <= non_nil(JTtbl.JT_MADOGEAR_B)) or
        (job_id >= non_nil(JTtbl.JT_PORING_NOVICE_B) and job_id <= non_nil(JTtbl.JT_DOG_CHASER_B)) or
        job_id == non_nil(JTtbl.JT_SUPERNOVICE2_B) or job_id == non_nil(JTtbl.JT_PORING_SNOVICE2_B) or
        job_id == non_nil(JTtbl.JT_FOX_WIZ_B) or job_id == non_nil(JTtbl.JT_PIG_BLACKSMITH_B) or
        (job_id >= non_nil(JTtbl.JT_PIG_MECHANIC_B) and job_id <= non_nil(JTtbl.JT_LION_CRUSADER_B)) or
        (job_id >= non_nil(JTtbl.JT_NINJA_B) and job_id <= non_nil(JTtbl.JT_REBELLION_B)) or
        job_id == non_nil(JTtbl.JT_STAR_EMPEROR_B) or job_id == non_nil(JTtbl.JT_SOUL_REAPER_B) or
        job_id == non_nil(JTtbl.JT_STAR2_EMPEROR_B) or job_id == non_nil(JTtbl.JT_HAETAE_STAR_EMPEROR_B) or
        job_id == non_nil(JTtbl.JT_HAETAE_SOUL_REAPER_B)
end
