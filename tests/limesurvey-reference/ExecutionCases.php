<?php
/** Build a source snapshot with explicit, executable options in the disposable database. */
class ExecutionCases
{
    public static function run($input, $output)
    {
        if (getenv('RUSTSURVEY_REFERENCE_ONLY') !== '1') {
            throw new RuntimeException('Execution cases require the isolated reference harness.');
        }
        $result = XMLImportSurvey($input);
        $id = (int) ($result['newsid'] ?? 0);
        if (!$id || !empty($result['error'])) {
            throw new RuntimeException('Synthetic execution import failed.');
        }
        $flags = ['anonymized', 'savetimings', 'datestamp', 'usecookie', 'allowregister',
            'allowsave', 'autoredirect', 'allowprev', 'printanswers', 'ipaddr', 'ipanonymize',
            'refurl', 'publicstatistics', 'publicgraphs', 'listpublic', 'htmlemail',
            'sendconfirmation', 'tokenanswerspersistence', 'assessments', 'showxquestions',
            'shownoanswer', 'showwelcome', 'showprogress', 'nokeyboard', 'alloweditaftercompletion'];
        $options = array_fill_keys($flags, 'N');
        foreach (['datestamp', 'allowsave', 'allowprev', 'shownoanswer', 'showwelcome', 'showprogress'] as $name) {
            $options[$name] = 'Y';
        }
        $options = array_merge($options, ['format' => 'G', 'showgroupinfo' => 'B',
            'showqnumcode' => 'X', 'questionindex' => 0, 'navigationdelay' => 0]);
        $group = new SurveysGroups();
        $group->gsid = 9021;
        $group->name = 'reference_execution';
        $group->title = 'Synthetic execution';
        $group->parent_id = null;
        $group->sortorder = 0;
        $group->owner_id = 1;
        $group->created_by = 1;
        $group->save(false);
        $settings = new SurveysGroupsettings();
        $settings->attributes = SurveysGroupsettings::model()->findByPk(0)->attributes;
        $settings->gsid = 9021;
        foreach ($options as $name => $value) {
            $settings->$name = $value;
        }
        $settings->save(false);
        $values = array_merge($options, ['gsid' => 9021, 'template' => 'vanilla', 'usecaptcha' => 'N',
            'expires' => null, 'startdate' => null, 'emailresponseto' => '', 'emailnotificationto' => '',
            'showsurveypolicynotice' => 0, 'bounceprocessing' => 'N', 'googleanalyticsstyle' => 0]);
        Yii::app()->db->createCommand()->update(Survey::model()->tableName(), $values, 'sid=:id', [':id' => $id]);
        // This fixture deliberately excludes plugin execution; no existing installation is touched.
        Yii::app()->db->createCommand()->update(Plugin::model()->tableName(), ['active' => 0]);
        Survey::model()->resetCache();
        $survey = Survey::model()->findByPk($id);
        $globalNoAnswer = (int) Yii::app()->getConfig('shownoanswer');
        $facts = ['schemaVersion' => 1, 'surveyId' => (string) $id,
            'hasParticipantsTable' => (bool) $survey->hasTokensTable,
            'deleteNonValues' => (bool) Yii::app()->getConfig('deletenonvalues'),
            'showNoAnswer' => $globalNoAnswer === 1 || ($globalNoAnswer === 2 && $survey->oOptions->shownoanswer === 'Y'),
            'effectiveTheme' => $survey->templateEffectiveName,
            'activePlugins' => (int) Plugin::model()->countByAttributes(['active' => 1]),
            'timeZone' => date_default_timezone_get(), 'timeAdjust' => (string) Yii::app()->getConfig('timeadjust')];
        require_once __DIR__ . '/RustSurveySettingsCommand.php';
        $helper = new RustSurveySettingsCommand('rustsurveysettings', Yii::app());
        $helper->run([(string) $id, $output . '/execution-context.json']);
        require_once __DIR__ . '/RustSurveyRuntimeCommand.php';
        $runtime = new RustSurveyRuntimeCommand('rustsurveyruntime', Yii::app());
        $runtime->run([(string) $id, $output . '/execution-facts.json']);
        $bytes = file_get_contents($output . '/execution-facts.json');
        if (json_decode($bytes, true, 512, JSON_THROW_ON_ERROR) !== $facts) {
            throw new RuntimeException('Extracted runtime facts differ from the full source model.');
        }
        $refused = false;
        try {
            $runtime->run([(string) $id, $output . '/execution-facts.json']);
        } catch (RuntimeException $error) {
            $refused = true;
        }
        if (!$refused || file_get_contents($output . '/execution-facts.json') !== $bytes) {
            throw new RuntimeException('Runtime facts were overwritten.');
        }
        self::write($output . '/execution.lss', surveyGetXMLData($id));
    }

    private static function write($path, $bytes)
    {
        if (!is_string($bytes) || file_put_contents($path, $bytes) === false) {
            throw new RuntimeException('Synthetic execution artifact could not be written.');
        }
    }
}
