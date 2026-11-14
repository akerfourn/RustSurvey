<?php
/** Exercise default creation, field mapping and initialization in the pinned engine. */
class DefaultCases
{
    public static function run($input, $output)
    {
        $bytes = file_get_contents(dirname($input) . '/default-cases.json');
        if ($bytes === false || strlen($bytes) > 1048576) {
            throw new RuntimeException('Synthetic default manifest is missing or too large.');
        }
        $cases = json_decode($bytes, true, 64, JSON_THROW_ON_ERROR)['cases'];
        if (count($cases) > 64) {
            throw new RuntimeException('Synthetic default corpus is too large.');
        }
        $results = [];
        foreach ($cases as $index => $case) {
            $survey = self::create($input, $case);
            if (file_put_contents($output . '/defaults-' . $index . '.lss', surveyGetXMLData($survey->sid)) === false) {
                throw new RuntimeException('Default structure export failed.');
            }
            $values = [];
            $writes = [];
            $previous = null;
            foreach ($case['steps'] as $step) {
                $result = self::initialize($survey, $case, $step, $previous);
                $previous = $result['value'];
                $values[] = $previous;
                $writes[] = $result['write'];
            }
            $results[] = ['name' => $case['name'], 'values' => $values, 'writes' => $writes];
        }
        if (file_put_contents($output . '/default-results.json', json_encode($results, JSON_THROW_ON_ERROR)) === false) {
            throw new RuntimeException('Default results could not be written.');
        }
        self::runNative(dirname($input), $output, $cases);
    }

    private static function runNative($directory, $output, $cases)
    {
        $bytes = file_get_contents($directory . '/native-default-cases.json');
        if ($bytes === false || strlen($bytes) > 1048576) {
            throw new RuntimeException('Native default manifest is missing or too large.');
        }
        $indices = json_decode($bytes, true, 64, JSON_THROW_ON_ERROR);
        if (count($indices) > 64) {
            throw new RuntimeException('Native default corpus is too large.');
        }
        $results = [];
        foreach ($indices as $index) {
            if (!is_int($index) || !isset($cases[$index])) {
                throw new RuntimeException('Invalid native default case index.');
            }
            $case = $cases[$index];
            $import = XMLImportSurvey($directory . '/native-defaults-' . $index . '.lss');
            if (!empty($import['error']) || empty($import['newsid'])) {
                throw new RuntimeException('Native default import failed.');
            }
            $survey = Survey::model()->findByPk((int) $import['newsid']);
            if (file_put_contents($output . '/native-defaults-' . $index . '.lss', surveyGetXMLData($survey->sid)) === false) {
                throw new RuntimeException('Native default export failed.');
            }
            $values = [];
            $writes = [];
            $previous = null;
            foreach ($case['steps'] as $step) {
                $result = self::initialize($survey, $case, $step, $previous);
                $previous = $result['value'];
                $values[] = $previous;
                $writes[] = $result['write'];
            }
            $results[] = ['name' => $case['name'], 'values' => $values, 'writes' => $writes];
        }
        if (file_put_contents($output . '/native-default-results.json', json_encode($results, JSON_THROW_ON_ERROR)) === false) {
            throw new RuntimeException('Native default results could not be written.');
        }
    }

    private static function create($input, $case)
    {
        $import = XMLImportSurvey($input);
        if (!empty($import['error']) || empty($import['newsid'])) {
            throw new RuntimeException('Default reference import failed.');
        }
        $survey = Survey::model()->findByPk((int) $import['newsid']);
        Question::model()->updateAll(['relevance' => '1'], 'sid=:sid', [':sid' => $survey->sid]);
        $question = Question::model()->findByAttributes(['sid' => $survey->sid, 'title' => $case['question'], 'parent_qid' => 0]);
        if (!$question) {
            throw new RuntimeException('Default reference question is missing.');
        }
        $question->same_default = !empty($case['sameDefault']) ? 1 : 0;
        if (isset($case['relevance'])) {
            $question->relevance = $case['relevance'];
        }
        if (($case['special'] ?? '') === 'other') {
            $question->other = 'Y';
        }
        if (!$question->save()) {
            throw new RuntimeException('Default reference question update failed.');
        }
        if (isset($case['filter'])) {
            $attribute = new QuestionAttribute();
            $attribute->attributes = ['qid' => $question->qid, 'attribute' => 'array_filter', 'value' => $case['filter']];
            if (!$attribute->save()) {
                throw new RuntimeException('Default reference filter could not be created.');
            }
        }
        $subquestion = null;
        if (isset($case['row'])) {
            $subquestion = Question::model()->findByAttributes(['parent_qid' => $question->qid, 'title' => $case['row'], 'scale_id' => 0]);
            if (!$subquestion) {
                throw new RuntimeException('Default reference subquestion is missing.');
            }
        }
        $default = new DefaultValue();
        $default->attributes = ['qid' => $question->qid, 'sqid' => $subquestion ? $subquestion->qid : 0, 'scale_id' => 0, 'specialtype' => $case['special'] ?? ''];
        // The pinned admin controller assigns this field outside safe attributes.
        $default->specialtype = $case['special'] ?? '';
        if (!$default->save()) {
            throw new RuntimeException('Default reference value could not be created.');
        }
        foreach ($case['values'] as $language => $value) {
            $translation = new DefaultValueL10n();
            $translation->attributes = ['dvid' => $default->dvid, 'language' => $language, 'defaultvalue' => $value];
            if (!$translation->save()) {
                throw new RuntimeException('Default reference translation could not be created.');
            }
        }
        return $survey;
    }

    private static function initialize($survey, $case, $step, $previous)
    {
        $language = $step['language'];
        $session = 'survey_' . $survey->sid;
        $_SESSION['LEMsid'] = (int) $survey->sid;
        $_SESSION['LEMlang'] = $language;
        $_SESSION[$session] = ['s_lang' => $language, 'token' => ''];
        $fields = [];
        foreach (createFieldMap($survey, 'full', true, false, $language) as $field) {
            if (!empty($field['qid'])) {
                $fields[viewHelper::getFieldCode($field, ['LEMcompat' => true])] = $field['fieldname'];
            }
        }
        $target = $case['target'] ?? $case['question'];
        if (!isset($fields[$target])) {
            throw new RuntimeException('Default reference field is missing.');
        }
        LimeExpressionManager::StartSurvey($survey->sid, 'survey', ['active' => false, 'anonymized' => true, 'deletenonvalues' => true], true);
        LimeExpressionManager::StartProcessingPage(true, true);
        $seed = array_merge($case['seed'] ?? [], $step['seed'] ?? []);
        foreach ($seed as $alias => $_value) {
            if (!isset($fields[$alias])) {
                throw new RuntimeException('Default reference seed field is missing.');
            }
        }
        foreach ($fields as $alias => $field) {
            LimeExpressionManager::setValueToKnowVar($field, null);
            if (array_key_exists($alias, $seed)) {
                $_SESSION[$session][$field] = $seed[$alias];
                LimeExpressionManager::setValueToKnowVar($field, $seed[$alias]);
            }
        }
        $value = null;
        switch ($step['state']) {
            case 'missing':
            case 'null':
                break;
            case 'empty':
                $value = '';
                break;
            case 'keep':
                $value = $previous;
                break;
            case 'existing':
                $value = $step['value'];
                break;
            default:
                throw new RuntimeException('Unknown synthetic default state.');
        }
        $field = $fields[$target];
        if ($step['state'] !== 'missing') {
            $_SESSION[$session][$field] = $value;
            LimeExpressionManager::setValueToKnowVar($field, $value);
        }
        $manager = LimeExpressionManager::singleton();
        // Navigation clears this per page; direct validation probes must do the same.
        $cache = new ReflectionProperty(LimeExpressionManager::class, 'ParseResultCache');
        $cache->setValue($manager, []);
        $manager->ProcessAllNeededRelevance();
        foreach ($seed as $alias => $value) {
            if ((string) LimeExpressionManager::GetVarAttribute($fields[$alias], 'code', '', -1, -1) !== (string) $value) {
                throw new RuntimeException('Default reference seed binding failed.');
            }
        }
        $manager->_CreateSubQLevelRelevanceAndValidationEqns();
        $question = Question::model()->findByAttributes(['sid' => $survey->sid, 'title' => $case['question'], 'parent_qid' => 0]);
        $status = $manager->_ValidateQuestion(LimeExpressionManager::GetQuestionSeq($question->qid));
        $result = $_SESSION[$session][$field] ?? null;
        $write = ['present' => false];
        if (array_key_exists($field, $status['updatedValues'])) {
            $update = $status['updatedValues'][$field];
            $write = ['present' => true, 'value' => $update === null ? null : (string) $update['value']];
        }
        return ['value' => $result === null ? null : (string) $result, 'write' => $write];
    }
}
