use sqlx::MySqlPool;

use crate::api::routes::v1::notifications::{notifier, NotifierImpl};
use crate::application::results::ResultUseCases;
use crate::infrastructure::clock::SystemClock;
use crate::infrastructure::repositories::mysql_matches::MySqlMatchRepository;
use crate::infrastructure::repositories::mysql_pool_members::MySqlPoolMemberRepository;
use crate::infrastructure::repositories::mysql_pools::MySqlPoolRepository;
use crate::infrastructure::repositories::mysql_scoring_rules::MySqlScoringRuleRepository;
use crate::infrastructure::repositories::mysql_standings::MySqlStandingRepository;

pub type ResultUseCasesImpl = ResultUseCases<
    MySqlStandingRepository,
    MySqlMatchRepository,
    MySqlScoringRuleRepository,
    MySqlPoolRepository,
    MySqlPoolMemberRepository,
    SystemClock,
    NotifierImpl,
>;

pub fn result_use_cases(db: MySqlPool) -> ResultUseCasesImpl {
    ResultUseCases::new(
        MySqlStandingRepository::new(db.clone()),
        MySqlMatchRepository::new(db.clone()),
        MySqlScoringRuleRepository::new(db.clone()),
        MySqlPoolRepository::new(db.clone()),
        MySqlPoolMemberRepository::new(db.clone()),
        SystemClock,
        notifier(db),
    )
}
