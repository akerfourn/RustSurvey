<?php
/** Read-only source helper for the schema-623 settings transfer context. */
class RustSurveySettingsCommand extends CConsoleCommand
{
    private const OPTIONS = [
        'anonymized', 'savetimings', 'datestamp', 'usecookie', 'allowregister', 'allowsave',
        'autoredirect', 'allowprev', 'printanswers', 'ipaddr', 'ipanonymize', 'refurl',
        'publicstatistics', 'publicgraphs', 'listpublic', 'htmlemail', 'sendconfirmation',
        'tokenanswerspersistence', 'assessments', 'showxquestions', 'shownoanswer',
        'showwelcome', 'showprogress', 'nokeyboard', 'alloweditaftercompletion',
        'format', 'showgroupinfo', 'showqnumcode', 'questionindex', 'navigationdelay',
    ];

    public function run($args)
    {
        if (count($args) !== 2 || !ctype_digit($args[0]) || $args[0] === '0') {
            throw new RuntimeException('Usage: rustsurveysettings SURVEY_ID OUTPUT_JSON');
        }
        if (Yii::app()->getConfig('versionnumber') !== '6.5.14'
            || (int) Yii::app()->getConfig('DBVersion') !== 623) {
            throw new RuntimeException('This settings helper requires LimeSurvey 6.5.14 and database schema 623.');
        }
        $survey = Yii::app()->db->createCommand()->select(['sid', 'gsid'])
            ->from(Survey::model()->tableName())->where('sid=:sid', [':sid' => $args[0]])->queryRow();
        if (!$survey) {
            throw new RuntimeException('Source survey not found.');
        }
        $global = $this->settings(0);
        if (!$global) {
            throw new RuntimeException('Source global group settings are missing.');
        }
        $groups = [];
        $seen = [];
        if ($survey['gsid'] === null || !ctype_digit((string) $survey['gsid'])) {
            throw new RuntimeException('The source survey settings group is unknown.');
        }
        $id = $survey['gsid'];
        while ($id !== null && (string) $id !== '0') {
            if (isset($seen[$id]) || count($groups) >= 64) {
                throw new RuntimeException('Source settings group hierarchy is cyclic or exceeds 64 groups.');
            }
            $seen[$id] = true;
            $group = Yii::app()->db->createCommand()->select(['gsid', 'parent_id'])
                ->from(SurveysGroups::model()->tableName())->where('gsid=:id', [':id' => $id])->queryRow();
            $settings = $this->settings($id);
            if (!$group || !$settings) {
                throw new RuntimeException('A source settings group or ancestor is missing.');
            }
            $groups[] = ['id' => (string) $id,
                'parentId' => $group['parent_id'] === null ? null : (string) $group['parent_id'],
                'values' => $this->values($settings)];
            $id = $group['parent_id'];
        }
        $context = ['schemaVersion' => 1, 'surveyId' => (string) $survey['sid'],
            'surveyGroupId' => (string) $survey['gsid'], 'groups' => $groups,
            'global' => $this->values($global)];
        $bytes = json_encode($context, JSON_PRETTY_PRINT | JSON_UNESCAPED_UNICODE | JSON_THROW_ON_ERROR) . "\n";
        if (strlen($bytes) > 65536) {
            throw new RuntimeException('The source settings context exceeds 64 KiB.');
        }
        $this->writeNew($args[1], $bytes);
        return 0;
    }

    private function settings($id)
    {
        return Yii::app()->db->createCommand()->select(self::OPTIONS)
            ->from(SurveysGroupsettings::model()->tableName())->where('gsid=:id', [':id' => $id])->queryRow();
    }

    private function values($row)
    {
        $values = [];
        foreach (self::OPTIONS as $name) {
            $value = $row[$name];
            if ($value !== null && strlen((string) $value) > 64) {
                throw new RuntimeException('A source option exceeds the transfer limit.');
            }
            $values[$name] = $value === null ? null : (string) $value;
        }
        return $values;
    }

    protected function writeNew($path, $bytes)
    {
        $previousMask = umask(0077);
        try {
            $file = @fopen($path, 'x');
        } finally {
            umask($previousMask);
        }
        if ($file === false) {
            throw new RuntimeException('Cannot create the output context; existing files are never overwritten.');
        }
        $complete = false;
        try {
            if (PHP_OS_FAMILY !== 'Windows' && !chmod($path, 0600)) {
                throw new RuntimeException('Cannot protect the output context.');
            }
            $offset = 0;
            while ($offset < strlen($bytes)) {
                $written = fwrite($file, substr($bytes, $offset));
                if ($written === false || $written === 0) {
                    throw new RuntimeException('Cannot finish the output context.');
                }
                $offset += $written;
            }
            if (!fflush($file)) {
                throw new RuntimeException('Cannot flush the output context.');
            }
            $complete = true;
        } finally {
            fclose($file);
            if (!$complete) {
                unlink($path);
            }
        }
    }
}
