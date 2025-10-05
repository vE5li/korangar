-- Function to get the tab in the skill tree from the job id.
--
-- As far as I can tell, this is hard coded in the original client. Having it
-- in Lua allows modifying/extending this easily without editing the Korangar
-- source code.

-- TODO: We probably should use JTbl here instead, I believe that would make all jobs available.
function GET_TAB_FOR_JOB(job_id)
    local non_nil = function(job_id)
        return job_id or -1
    end

    if
        job_id == non_nil(JOBID.JT_NOVICE)
    then
        return 0
    elseif
        job_id < non_nil(JOBID.JT_KNIGHT) or
        job_id == non_nil(JOBID.JT_TAEKWON) or
        (job_id >= non_nil(JOBID.JT_SUPERNOVICE) and job_id <= non_nil(JOBID.JT_NINJA)) or
        job_id == non_nil(JOBID.JT_DO_SUMMONER) or
        (job_id <= non_nil(JOBID.JT_THIEF_H) and job_id >= non_nil(JOBID.JT_NOVICE_H)) or
        (job_id >= non_nil(JOBID.JT_NOVICE_B) and job_id <= non_nil(JOBID.JT_THIEF_B)) or
        job_id == non_nil(JOBID.JT_DO_SUMMONER_B) or
        job_id == non_nil(JOBID.JT_NINJA_B) or
        job_id == non_nil(JOBID.JT_TAEKWON_B) or
        job_id == non_nil(JOBID.JT_GUNSLINGER_B)
    then
        return 1
    elseif
        job_id < non_nil(JOBID.JT_NOVICE_H) or
        job_id == non_nil(JOBID.JT_STAR) or
        job_id == non_nil(JOBID.JT_LINKER) or
        (job_id >= non_nil(JOBID.JT_KAGEROU) and job_id <= non_nil(JOBID.JT_REBELLION)) or
        job_id == non_nil(JOBID.JT_SUPERNOVICE2) or
        job_id == non_nil(JOBID.JT_SPIRIT_HANDLER) or
        job_id < non_nil(JOBID.JT_RUNE_KNIGHT) or
        (job_id >= non_nil(JOBID.JT_KNIGHT_B) and job_id <= non_nil(JOBID.JT_DANCER_B)) or
        (job_id >= non_nil(JOBID.JT_KAGEROU_B) and job_id <= non_nil(JOBID.JT_REBELLION_B))
    then
        return 2
    elseif
        job_id < non_nil(JOBID.JT_DRAGON_KNIGHT) or
        job_id == non_nil(JOBID.JT_STAR_EMPEROR) or
        job_id == non_nil(JOBID.JT_SOUL_REAPER) or
        (job_id >= non_nil(JOBID.JT_RUNE_KNIGHT_B) and job_id <= non_nil(JOBID.JT_SHADOW_CHASER_B)) or
        job_id == non_nil(JOBID.JT_EMPEROR_B) or
        job_id == non_nil(JOBID.JT_REAPER_B)
    then
        return 3
    elseif
        job_id <= non_nil(JOBID.JT_TROUVERE) or
        (job_id >= non_nil(JOBID.JT_SKY_EMPEROR) and job_id <= non_nil(JOBID.JT_HYPER_NOVICE))
    then
        return 4
    else
        -- Fallback
        return 0
    end
end
