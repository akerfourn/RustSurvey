<?php
require_once __DIR__ . '/RustSurveySettingsCommand.php';

/** Read-only runtime facts missing from an LSS; no participant or response row is read. */
class RustSurveyRuntimeCommand extends RustSurveySettingsCommand
{
    public function run($args)
    {
        if (count($args) !== 2 || !ctype_digit($args[0]) || $args[0] === '0') {
            throw new RuntimeException('Usage: rustsurveyruntime SURVEY_ID OUTPUT_JSON');
        }
        if (Yii::app()->getConfig('versionnumber') !== '6.5.14'
            || (int) Yii::app()->getConfig('DBVersion') !== 623) {
            throw new RuntimeException('This runtime helper requires LimeSurvey 6.5.14 and database schema 623.');
        }
        Yii::app()->loadHelper('common');
        Yii::app()->loadHelper('globalsettings');
        $survey = Survey::model()->findByPk($args[0], ['select' => ['sid', 'gsid', 'shownoanswer', 'template']]);
        if (!$survey) {
            throw new RuntimeException('Source survey not found.');
        }
        $noAnswer = (int) Yii::app()->getConfig('shownoanswer');
        $facts = ['schemaVersion' => 1, 'surveyId' => (string) $survey->sid,
            'hasParticipantsTable' => (bool) $survey->hasTokensTable,
            'deleteNonValues' => (bool) Yii::app()->getConfig('deletenonvalues'),
            'showNoAnswer' => $noAnswer === 1 || ($noAnswer === 2 && $survey->oOptions->shownoanswer === 'Y'),
            'effectiveTheme' => $survey->templateEffectiveName,
            'activePlugins' => (int) Plugin::model()->countByAttributes(['active' => 1]),
            'timeZone' => date_default_timezone_get(), 'timeAdjust' => (string) Yii::app()->getConfig('timeadjust')];
        $bytes = json_encode($facts, JSON_PRETTY_PRINT | JSON_UNESCAPED_UNICODE | JSON_THROW_ON_ERROR) . "\n";
        if (strlen($bytes) > 65536) {
            throw new RuntimeException('The runtime facts exceed 64 KiB.');
        }
        $this->writeNew($args[1], $bytes);
        return 0;
    }
}
