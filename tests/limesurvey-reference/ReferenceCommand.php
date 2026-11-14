<?php
/** Exercise the actual pinned import/export implementation with synthetic fixtures. */
class ReferenceCommand extends CConsoleCommand
{
    public function run($args)
    {
        if (getenv('RUSTSURVEY_REFERENCE_ONLY') !== '1' || count($args) !== 2) {
            throw new RuntimeException('Expected synthetic input LSS and output directory.');
        }
        $input = $args[0];
        $output = $args[1];
        if (!is_file($input) || !is_dir($output)) {
            throw new RuntimeException('Reference fixture paths are missing.');
        }
        Yii::app()->session['loginID'] = 1;
        Yii::app()->loadHelper('common');
        Yii::app()->loadHelper('globalsettings');
        Yii::app()->loadHelper('replacements');
        Yii::app()->loadHelper('expressions/em_manager');
        Yii::app()->loadHelper('admin/import');
        Yii::app()->loadHelper('export');
        $result = XMLImportSurvey($input);
        if (!empty($result['error']) || empty($result['newsid'])) {
            throw new RuntimeException('Reference import failed: ' . json_encode($result));
        }
        $surveyId = (int) $result['newsid'];
        $xml = surveyGetXMLData($surveyId);
        if (!is_string($xml) || file_put_contents($output . '/roundtrip.lss', $xml) === false) {
            throw new RuntimeException('Reference export failed.');
        }
        $survey = Survey::model()->findByPk($surveyId);
        $questions = Question::model()->findAllByAttributes(['sid' => $surveyId, 'parent_qid' => 0]);
        $types = [];
        foreach ($questions as $question) {
            $types[$question->title] = $question->type;
        }
        ksort($types);
        Yii::import('application.helpers.viewHelper', true);
        $bindings = [];
        foreach (createFieldMap($survey, 'full', true, false, $survey->language) as $field) {
            // Information questions have an unused physical column, but no response cell.
            if (!empty($field['qid']) && $field['type'] !== 'X') {
                $bindings[viewHelper::getFieldCode($field, ['LEMcompat' => true])] = $field['fieldname'];
            }
        }
        ksort($bindings);
        $summary = [
            'referenceCommit' => 'ac3c44744e2c3769b3941c41e50b68419423ce67',
            'version' => Yii::app()->getConfig('versionnumber'),
            'schema' => Yii::app()->getConfig('dbversionnumber'),
            'surveyId' => $surveyId,
            'questionTypes' => $types,
            'languages' => $survey->getAllLanguages(),
            'import' => $result,
            'bindings' => $bindings,
        ];
        Yii::app()->loadHelper('admin/activate');
        $activation = (new SurveyActivator($survey))->activate();
        if (($activation['status'] ?? null) !== 'OK') {
            throw new RuntimeException('Reference activation failed: ' . json_encode($activation));
        }
        $responses = XMLImportResponses(dirname($input) . '/responses.lsr', $surveyId, $result['FieldReMap']);
        if (!empty($responses['error']) || $responses['responses'] !== 2) {
            throw new RuntimeException('Reference response import failed: ' . json_encode($responses));
        }
        $summary['lsrResponses'] = [];
        foreach (Response::model($surveyId)->findAll() as $response) {
            $summary['lsrResponses'][] = $response->attributes;
        }
        $vv = CSVImportResponses(dirname($input) . '/responses.vv', $surveyId, ['sExistingId' => 'replaceanswers', 'bForceImport' => false, 'bNotFinalized' => false]);
        if (!empty($vv['errors'])) {
            throw new RuntimeException('Reference VV import failed: ' . json_encode($vv));
        }
        $summary['responses'] = [];
        foreach (Response::model($surveyId)->findAll() as $response) {
            $summary['responses'][] = $response->attributes;
        }
        $exported = getXMLDataSingleTable($surveyId, 'survey_' . $surveyId, 'Responses', 'responses', $output . '/roundtrip.lsr');
        if (!$exported) {
            throw new RuntimeException('Reference response export failed.');
        }
        $otherImport = XMLImportSurvey($input);
        $otherId = (int) ($otherImport['newsid'] ?? 0);
        if (!$otherId || $otherId === $surveyId) {
            throw new RuntimeException('Reference second survey import failed.');
        }
        $otherSurvey = Survey::model()->findByPk($otherId);
        $otherActivation = (new SurveyActivator($otherSurvey))->activate();
        if (($otherActivation['status'] ?? null) !== 'OK') {
            throw new RuntimeException('Reference second activation failed.');
        }
        $otherVv = CSVImportResponses(dirname($input) . '/campaign.vv', $otherId, ['sExistingId' => 'ignore', 'bForceImport' => false, 'bNotFinalized' => false]);
        if (!empty($otherVv['errors'])) {
            throw new RuntimeException('Reference campaign VV import failed: ' . json_encode($otherVv));
        }
        $summary['campaignBindings'] = [];
        foreach (createFieldMap($otherSurvey, 'full', true, false, $otherSurvey->language) as $field) {
            if (!empty($field['qid']) && $field['type'] !== 'X') {
                $summary['campaignBindings'][viewHelper::getFieldCode($field, ['LEMcompat' => true])] = $field['fieldname'];
            }
        }
        $summary['campaignResponses'] = [];
        foreach (Response::model($otherId)->findAll() as $response) {
            $summary['campaignResponses'][] = $response->attributes;
        }
        $summary['navigation'] = [];
        $exclusiveImport = XMLImportSurvey(dirname($input) . '/exclusive.lss');
        if (!empty($exclusiveImport['error']) || empty($exclusiveImport['newsid'])) {
            throw new RuntimeException('Reference exclusive options import failed.');
        }
        $summary['exclusiveAttributes'] = [];
        foreach (Question::model()->findAllByAttributes(['sid' => (int) $exclusiveImport['newsid'], 'parent_qid' => 0]) as $question) {
            foreach (QuestionAttribute::model()->findAllByAttributes(['qid' => $question->qid]) as $attribute) {
                if (in_array($attribute->attribute, ['exclude_all_others', 'exclude_all_others_auto'], true)) {
                    $summary['exclusiveAttributes'][$question->title][$attribute->attribute] = $attribute->value;
                }
            }
        }
        foreach (['group', 'question', 'all'] as $format) {
            $navigationImport = XMLImportSurvey(dirname($input) . '/navigation-' . $format . '.lss');
            if (!empty($navigationImport['error']) || empty($navigationImport['newsid'])) {
                throw new RuntimeException('Reference navigation import failed.');
            }
            $navigationSurvey = Survey::model()->findByPk((int) $navigationImport['newsid']);
            $summary['navigation'][$format] = [];
            foreach (['format', 'showprogress', 'allowprev'] as $name) {
                $summary['navigation'][$format][$name] = $navigationSurvey->oOptions->$name;
            }
        }
        if (file_put_contents($output . '/summary.json', json_encode($summary, JSON_PRETTY_PRINT | JSON_UNESCAPED_UNICODE)) === false) {
            throw new RuntimeException('Reference summary could not be written.');
        }
        require_once __DIR__ . '/SettingsCases.php';
        require_once __DIR__ . '/FilterCases.php';
        require_once __DIR__ . '/DefaultCases.php';
        FilterCases::run(dirname($input), $output);
        DefaultCases::run($input, $output);
        SettingsCases::run($otherSurvey, $output);
        require_once __DIR__ . '/ExecutionCases.php';
        ExecutionCases::run($input, $output);
        echo "Reference import/export and settings comparisons completed.\n";
        return 0;
    }
}
