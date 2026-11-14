<?php
/** Synthetic comparisons of actual SurveysGroupsettings inheritance. */
class SettingsCases
{
    public static function run($survey, $output)
    {
        if (getenv('RUSTSURVEY_REFERENCE_ONLY') !== '1') {
            throw new RuntimeException('Settings cases require the isolated reference harness.');
        }
        $flags = ['anonymized', 'savetimings', 'datestamp', 'usecookie', 'allowregister',
            'allowsave', 'autoredirect', 'allowprev', 'printanswers', 'ipaddr', 'ipanonymize',
            'refurl', 'publicstatistics', 'publicgraphs', 'listpublic', 'htmlemail',
            'sendconfirmation', 'tokenanswerspersistence', 'assessments', 'showxquestions',
            'shownoanswer', 'showwelcome', 'showprogress', 'nokeyboard', 'alloweditaftercompletion'];
        $defaults = array_fill_keys($flags, 'N');
        $defaults = array_merge($defaults, ['showwelcome' => 'Y', 'format' => 'G',
            'showgroupinfo' => 'B', 'showqnumcode' => 'X', 'questionindex' => '0', 'navigationdelay' => '0']);
        $global = SurveysGroupsettings::model()->findByPk(0);
        if (!$global) {
            throw new RuntimeException('Synthetic global settings are missing.');
        }
        foreach ($defaults as $name => $value) {
            $global->$name = $value;
        }
        $global->save(false);
        if (!SurveysGroupsettings::model()->findByPk(0)) {
            throw new RuntimeException('Synthetic global settings could not be saved.');
        }
        foreach ([9011 => null, 9012 => 9011] as $id => $parent) {
            if (SurveysGroups::model()->findByPk($id)) {
                throw new RuntimeException('Synthetic settings group identity is already used.');
            }
            $group = new SurveysGroups();
            $group->gsid = $id;
            $group->name = 'reference_settings_' . $id;
            $group->title = 'Synthetic settings';
            $group->sortorder = 0;
            $group->owner_id = 1;
            $group->created_by = 1;
            $group->parent_id = $parent;
            if (!$group->save(false)) {
                throw new RuntimeException('Synthetic settings group could not be saved.');
            }
            $settings = new SurveysGroupsettings();
            $settings->attributes = $global->attributes;
            $settings->gsid = $id;
            foreach ($defaults as $name => $_) {
                $settings->$name = in_array($name, ['navigationdelay', 'questionindex']) ? -1 : 'I';
            }
            $settings->showwelcome = $id === 9011 ? 'N' : 'I';
            $settings->format = $id === 9012 ? 'A' : 'I';
            $settings->navigationdelay = $id === 9011 ? 4 : 0;
            $settings->questionindex = $id === 9011 ? 2 : 0;
            $settings->save(false);
            if (!SurveysGroupsettings::model()->findByPk($id)) {
                throw new RuntimeException('Synthetic group settings could not be saved.');
            }
        }
        $cases = [
            ['gsid' => 9012, 'showwelcome' => 'I', 'format' => 'I', 'navigationdelay' => 0, 'questionindex' => 0],
            ['gsid' => 9012, 'showwelcome' => 'Y', 'format' => 'S', 'navigationdelay' => 3, 'questionindex' => 1],
            ['gsid' => 0, 'showwelcome' => 'N', 'format' => 'S', 'navigationdelay' => 7, 'questionindex' => 2],
            ['gsid' => 9012, 'showwelcome' => '', 'format' => 'I', 'navigationdelay' => -1, 'questionindex' => -1],
        ];
        foreach ($cases as $index => $values) {
            Yii::app()->db->createCommand()->update(Survey::model()->tableName(), $values, 'sid=:sid', [':sid' => $survey->sid]);
            Survey::model()->resetCache();
            $actual = Survey::model()->findByPk($survey->sid);
            if ((int) $actual->gsid !== $values['gsid']) {
                throw new RuntimeException('Synthetic survey group assignment was not read back.');
            }
            $context = ['schemaVersion' => 1, 'surveyId' => (string) $actual->sid,
                'surveyGroupId' => (string) $actual->gsid, 'groups' => [],
                'global' => self::selected($global, array_keys($defaults))];
            foreach ([9012, 9011] as $id) {
                $settings = SurveysGroupsettings::model()->findByPk($id);
                $context['groups'][] = ['id' => (string) $id, 'parentId' => $id === 9012 ? '9011' : null,
                    'values' => self::selected($settings, array_keys($defaults))];
            }
            $expected = [];
            foreach ($defaults as $name => $_) {
                $expected[$name] = (string) $actual->oOptions->$name;
            }
            self::write($output . '/settings-' . $index . '.lss', surveyGetXMLData($actual->sid));
            self::write($output . '/settings-' . $index . '-context.json', json_encode($context));
            self::write($output . '/settings-' . $index . '-expected.json', json_encode($expected));
            require_once __DIR__ . '/RustSurveySettingsCommand.php';
            $helper = new RustSurveySettingsCommand('rustsurveysettings', Yii::app());
            $helper->run([(string) $actual->sid, $output . '/settings-' . $index . '-extracted.json']);
        }
    }

    private static function selected($model, $names)
    {
        $values = [];
        foreach ($names as $name) {
            $values[$name] = $model->$name === null ? null : (string) $model->$name;
        }
        return $values;
    }

    private static function write($path, $content)
    {
        if (!is_string($content) || file_put_contents($path, $content) === false) {
            throw new RuntimeException('Synthetic settings artifact could not be written.');
        }
    }
}
