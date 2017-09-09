package xyz.clieb.bodygraph

import org.apache.poi.ss.usermodel.WorkbookFactory
import xyz.clieb.bodygraph.Closable._

import java.nio.file.{Path, Paths}
import java.time.{LocalDate, ZoneId}

import scala.util.{Failure, Success}

object Main {
  def main(args: Array[String]): Unit = {
    println(new Main().readFile(Paths.get("C:", "Users", "Chris", "My Tresors", "Official Documents", "frohman", "Body Tracker.xlsx")))
  }
}

class Main {
  def readFile(path: Path): Seq[Record] = {
    closable(WorkbookFactory.create(path.toFile)) { workbook =>
      val sheet = workbook.getSheetAt(0)
      (sheet.getFirstRowNum + 1 to sheet.getLastRowNum)
        .map { case rowNum: Int =>
          val row = sheet.getRow(rowNum)
          Record(
            row.getCell(0).getDateCellValue.toInstant.atZone(ZoneId.systemDefault()).toLocalDate,
            Option(row.getCell(2)).map(_.getNumericCellValue.toFloat),
            Option(row.getCell(3)).map(_.getNumericCellValue.toFloat),
            Option(row.getCell(4)).map(_.getNumericCellValue.toFloat),
            Option(row.getCell(5)).map(_.getNumericCellValue.toFloat),
            Option(row.getCell(6)).map(_.getNumericCellValue.toFloat),
            Option(row.getCell(7)).map(_.getNumericCellValue.toFloat),
          )
        }
    } match {
      case Success(value) => value
      case Failure(e) => throw e
    }
  }
}

case class Record(
    date: LocalDate,
    weight: Option[Float],
    fatWeight: Option[Float],
    pctFat: Option[Float],
    pctWater: Option[Float],
    pctBone: Option[Float],
    bmi: Option[Float])
